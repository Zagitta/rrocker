use crate::pipe::Pipe;
use anyhow::{Context, Result};
use nix::{
    sched::{self, CloneCb, CloneFlags},
    sys::signal::Signal::SIGCHLD,
    unistd::Pid,
};
use serde::{de::DeserializeOwned, Serialize};
use std::{fs::File, io::BufReader, marker::PhantomData, ops::DerefMut};

const STACK_SIZE: usize = 1024 * 1024; //1MiB
///Used to hold our
pub struct CloneContext<'a, T: Serialize + DeserializeOwned + Send> {
    stack: Box<[u8; STACK_SIZE]>,
    func: CloneCb<'a>,
    res_reader: File,
    phantom: PhantomData<T>,
}

impl<'a, T: Serialize + DeserializeOwned + Send> CloneContext<'a, T> {
    pub fn new<F: 'a + FnMut() -> Result<T>>(mut func: F) -> Result<Self> {
        let (res_reader, res_writer) = Pipe::new()?.split();
        Ok(Self {
            res_reader,
            stack: Box::new([0u8; STACK_SIZE]),
            func: Box::new(move || {
                //this is quite an abomination because anyhow errors don't impl Serialize
                //so we use the serde_error crate to magically wrap it.
                //It's ugly but allows us much easier insight into what went wrong on the clone side.
                //In a production system this wouldn't be an issue since you'd make proper Error enums
                let res = func().map_err(|e| serde_error::Error::new(&*e));

                if let Err(_e) = bincode::serialize_into(&res_writer, &res) {
                    1
                } else {
                    0
                }
            }),
            phantom: PhantomData::default(),
        })
    }

    pub fn execute(mut self) -> Result<(Pid, ResultReader<T>)> {
        let pid = sched::clone(
            self.func,
            self.stack.deref_mut(),
            //it's of UTMOST importance CLONE_VM is __NOT__ specified here
            //as that gives the child process write access to the daemon
            CloneFlags::CLONE_NEWPID
                | CloneFlags::CLONE_NEWNS
                | CloneFlags::CLONE_NEWNET
                | CloneFlags::CLONE_NEWUSER
                | CloneFlags::CLONE_NEWUTS
                | CloneFlags::CLONE_NEWCGROUP,
            Some(SIGCHLD as i32),
        )
        .context("Failed to call clone()")?;

        Ok((pid, ResultReader::new(self.res_reader)))
    }
}

pub struct ResultReader<T> {
    reader: BufReader<File>,
    phantom: PhantomData<T>,
}

impl<T: DeserializeOwned> ResultReader<T> {
    pub fn new(reader: File) -> Self {
        Self {
            reader: BufReader::new(reader),
            phantom: Default::default(),
        }
    }

    //Block and wait for the result
    pub fn get_result(&mut self) -> Result<T> {
        bincode::deserialize_from::<_, std::result::Result<T, serde_error::Error>>(&mut self.reader)
            .context("Failed to deserialize inner Result")?
            .map_err(anyhow::Error::from)
    }
}

#[cfg(test)]
mod test {
    use nix::sys::wait::WaitStatus;
    use serde::{Deserialize, Serialize};

    use super::*;

    #[test]
    fn returns_data_and_exists() {
        #[derive(Debug, Serialize, Deserialize, Clone, PartialEq, Eq)]
        struct ComplexStruct {
            name: String,
            tup: (u32, u32),
        }

        let data = ComplexStruct {
            name: "Some test string to see this works".to_owned(),
            tup: (1234, 567890),
        };

        let cc = CloneContext::new(|| -> Result<ComplexStruct> {
            //this is a little bit sketchy but what happens is that this
            //lambda captures a reference to data.
            //And although this lambda gets executed in a child process
            //with isolated namespaces the clone(2) call works similarly to
            //fork(2) where the child process still has access to the parent's
            //memory address space and as such can access the reference and
            //clone the data which gets serialized and sent back over the
            //result pipe
            Ok(data.clone())
        })
        .unwrap();

        let (pid, mut rr) = cc.execute().unwrap();

        let res = rr.get_result();

        match res {
            Ok(s) => assert_eq!(s, data),
            Err(e) => assert!(false, "{:?}", e),
        }

        let wait_res = nix::sys::wait::waitpid(pid, None);

        assert_eq!(wait_res, Ok(WaitStatus::Exited(pid, 0)));
    }
}
