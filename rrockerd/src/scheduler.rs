use crate::auth::ClientAuth;
use dashmap::{
    mapref::one::{Ref, RefMut},
    DashMap,
};
use futures::{Stream, StreamExt};
use rrocker_lib::api::{
    scheduler_server::Scheduler, OutputStream, QueryTaskReply, StartTaskReply, StartTaskRequest,
    TaskHandle, TaskOutputReply, TaskState, TaskStatus,
};
use std::{collections::HashSet, pin::Pin};
use tokio::sync::broadcast::{channel, Receiver, Sender};
use tokio_stream::wrappers::BroadcastStream;
use tonic::{Response, Status};
use uuid::Uuid;

#[derive(Debug)]
struct Task {
    tx: Sender<(String, OutputStream)>,
} //todo

impl Task {
    pub fn new() -> Self {
        let (tx, _rx) = channel(32);
        Self { tx }
    }
    pub fn subscribe(&self) -> Receiver<(String, OutputStream)> {
        self.tx.subscribe()
    }
}

#[derive(Debug, Default)]
struct SchedulerServer {
    task_map: DashMap<Uuid, Task>,
    client_tasks: DashMap<String, HashSet<Uuid>>,
}

const ADMIN_GROUP: &str = "admin";

impl SchedulerServer {
    fn verify_task_access(&self, auth: &ClientAuth, uuid: &Uuid) -> bool {
        if auth.group == ADMIN_GROUP {
            return true;
        }
        if let Some(set) = self.client_tasks.get(&auth.id) {
            if set.contains(uuid) {
                return true;
            }
        }
        false
    }

    /// Returns an iterator over a specific user's tasks.
    fn iter_tasks<'a>(
        &'a self,
        auth: &ClientAuth,
    ) -> impl Iterator<Item = Ref<'a, Uuid, Task>> + 'a {
        //We don't want to hold locks into task_map or client_tasks
        //for longer than necessary so collect/clone when needed.
        //This means the iterator won't see new tasks spawned
        //while iterating but that's ok
        let tasks = if auth.group == ADMIN_GROUP {
            self.task_map
                .iter()
                .map(|ent| *ent.key())
                .collect::<Vec<_>>()
        } else {
            self.client_tasks
                .get(&auth.id)
                .map(|e| e.clone())
                .unwrap_or_default()
                .into_iter()
                .collect::<Vec<_>>()
        };

        tasks
            .into_iter()
            .flat_map(move |uuid| self.task_map.get(&uuid))
    }

    /// Lookup a task based on it's handle while respecting the provided authorization
    fn lookup_task(&self, auth: &ClientAuth, uuid: &Uuid) -> Result<Ref<'_, Uuid, Task>, Status> {
        let task = self
            .task_map
            .get(uuid)
            .ok_or_else(|| Status::invalid_argument("Invalid task handle"))?;

        if self.verify_task_access(auth, uuid) {
            Ok(task)
        } else {
            Err(Status::invalid_argument("Invalid task handle"))
        }
    }
    /// Same as `lookup_task` but mut
    fn lookup_task_mut(
        &self,
        auth: &ClientAuth,
        uuid: &Uuid,
    ) -> Result<RefMut<'_, Uuid, Task>, Status> {
        let task = self
            .task_map
            .get_mut(uuid)
            .ok_or_else(|| Status::invalid_argument("Invalid task handle"))?;

        if self.verify_task_access(auth, uuid) {
            Ok(task)
        } else {
            Err(Status::invalid_argument("Invalid task handle"))
        }
    }

    fn new_task(
        &self,
        auth: &ClientAuth,
        _cmd: &str,
        _args: &[String],
    ) -> Result<Ref<'_, Uuid, Task>, Status> {
        let mut ent = self
            .task_map
            .entry(Uuid::new_v4())
            .or_insert_with(Task::new);
        {
            let (key, _task) = ent.pair_mut();
            self.client_tasks
                .entry(auth.id.clone())
                .or_default()
                .insert(*key);
        }

        //todo hookup worker

        Ok(ent.downgrade())
    }
}

fn request_to_auth<T>(req: &tonic::Request<T>) -> Result<&ClientAuth, Status> {
    req.extensions()
        .get::<ClientAuth>()
        .ok_or_else(|| Status::internal("Missing ClientAuth extension"))
}

fn string_to_uuid(uuid_string: &str) -> Result<Uuid, Status> {
    uuid_string
        .parse::<Uuid>()
        .map_err(|_| Status::invalid_argument("TaskHandle.uuid is not a valid UUIDv4"))
}

#[tonic::async_trait]
impl Scheduler for SchedulerServer {
    #[tracing::instrument]
    async fn start_task(
        &self,
        request: tonic::Request<StartTaskRequest>,
    ) -> Result<Response<StartTaskReply>, Status> {
        let _auth = request_to_auth(&request)?;

        todo!()
    }

    #[tracing::instrument]
    async fn stop_task(&self, request: tonic::Request<TaskHandle>) -> Result<Response<()>, Status> {
        let auth = request_to_auth(&request)?;
        let uuid = string_to_uuid(&request.get_ref().uuid)?;

        let _task = self.lookup_task_mut(auth, &uuid)?;

        todo!()
    }

    #[tracing::instrument]
    async fn query_task(
        &self,
        request: tonic::Request<TaskHandle>,
    ) -> Result<Response<QueryTaskReply>, Status> {
        let auth = request_to_auth(&request)?;
        let uuid = string_to_uuid(&request.get_ref().uuid)?;

        let _task = self.lookup_task(auth, &uuid)?;

        Ok(Response::new(QueryTaskReply {
            state: Some(TaskState {
                status: TaskStatus::TaskRunning.into(),
                code: 0,
            }),
        }))
    }

    type TaskOutputStreamStream =
        Pin<Box<dyn Stream<Item = Result<TaskOutputReply, Status>> + Send + Sync + 'static>>;

    #[tracing::instrument]
    async fn task_output_stream(
        &self,
        request: tonic::Request<TaskHandle>,
    ) -> Result<Response<Self::TaskOutputStreamStream>, Status> {
        let auth = request_to_auth(&request)?;
        let uuid = string_to_uuid(&request.get_ref().uuid)?;
        let task = self.lookup_task(auth, &uuid)?;

        //yikes
        let stream = BroadcastStream::new(task.subscribe())
            .filter_map(|item| async { item.ok() })
            .map(|(line, stream)| {
                Ok(TaskOutputReply {
                    line,
                    stream: stream.into(),
                })
            });
        let stream = Box::pin(stream) as Self::TaskOutputStreamStream;

        Ok(Response::new(stream))
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_verify_access() {
        let server = SchedulerServer::default();
        let a1 = ClientAuth {
            id: "a1".into(),
            group: ADMIN_GROUP.into(),
        };
        let c1 = ClientAuth {
            id: "c1".into(),
            group: "client".into(),
        };
        let c2 = ClientAuth {
            id: "c2".into(),
            group: "client".into(),
        };

        let ent1 = server.new_task(&c1, &"asd", &vec![]).unwrap();
        let ent2 = server.new_task(&c1, &"foo", &vec![]).unwrap();
        let ent3 = server.new_task(&c2, &"bar", &vec![]).unwrap();
        let ent4 = server.new_task(&c2, &"dsa", &vec![]).unwrap();

        //admin has access to everything
        assert_eq!(server.verify_task_access(&a1, &ent1.key()), true);
        assert_eq!(server.verify_task_access(&a1, &ent2.key()), true);
        assert_eq!(server.verify_task_access(&a1, &ent3.key()), true);
        assert_eq!(server.verify_task_access(&a1, &ent4.key()), true);

        //c1 has access to his own stuff
        assert_eq!(server.verify_task_access(&c1, &ent1.key()), true);
        assert_eq!(server.verify_task_access(&c1, &ent2.key()), true);
        //but not c2's tasks
        assert_eq!(server.verify_task_access(&c1, &ent3.key()), false);
        assert_eq!(server.verify_task_access(&c1, &ent4.key()), false);

        //and vice versa for c2
        assert_eq!(server.verify_task_access(&c2, &ent1.key()), false);
        assert_eq!(server.verify_task_access(&c2, &ent2.key()), false);
        assert_eq!(server.verify_task_access(&c2, &ent3.key()), true);
        assert_eq!(server.verify_task_access(&c2, &ent4.key()), true);
    }

    #[test]
    fn test_task_iter_access() {
        let server = SchedulerServer::default();
        let c1 = ClientAuth {
            id: "c1".into(),
            group: "client".into(),
        };
        let c2 = ClientAuth {
            id: "c2".into(),
            group: "client".into(),
        };
        let a1 = ClientAuth {
            id: "a1".into(),
            group: ADMIN_GROUP.into(),
        };

        server.new_task(&c1, &"asd", &vec![]).unwrap();
        assert_eq!(server.iter_tasks(&c1).count(), 1);
        assert_eq!(server.iter_tasks(&c2).count(), 0);
        assert_eq!(server.iter_tasks(&a1).count(), 1);
        server.new_task(&c2, &"foo", &vec![]).unwrap();
        assert_eq!(server.iter_tasks(&c1).count(), 1);
        assert_eq!(server.iter_tasks(&c2).count(), 1);
        assert_eq!(server.iter_tasks(&a1).count(), 2);
    }

    #[test]
    fn test_task_iter() {
        let server = SchedulerServer::default();
        let c1 = ClientAuth {
            id: "c1".into(),
            group: "client".into(),
        };

        let key1 = *server.new_task(&c1, &"asd", &vec![]).unwrap().key();
        server.new_task(&c1, &"foo", &vec![]).unwrap();

        let it = server.iter_tasks(&c1);
        server.task_map.remove(&key1);

        //This checks that although the iterator internally has a vec of [ent1, ent2]
        //removing ent1 from the task_map doesn't end the iterator prematurely when
        //it tries to lookup a removed task
        assert_eq!(it.count(), 1);
    }
}
