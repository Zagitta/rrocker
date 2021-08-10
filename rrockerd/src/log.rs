use futures::{Future, Stream};
use std::{
    pin::Pin,
    sync::{Arc, RwLock},
    task::{Context, Poll, Waker},
};

#[derive(Debug)]
struct SharedInternal<T> {
    items: Vec<Arc<T>>,
    wakers: Vec<Waker>,
    closed: bool,
}

impl<T> SharedInternal<T> {
    pub fn new() -> SharedInternal<T> {
        Self {
            items: Default::default(),
            wakers: Default::default(),
            closed: false,
        }
    }
}

#[derive(Debug)]
struct Shared<T> {
    inner: RwLock<SharedInternal<T>>,
}

#[derive(Debug)]
enum ReaderFut<T> {
    Ok(Option<Arc<T>>),
    Future { shared: Arc<Shared<T>>, idx: usize },
}
impl<T> Future for ReaderFut<T> {
    type Output = Option<Arc<T>>;

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let this = unsafe { self.get_unchecked_mut() };
        match this {
            ReaderFut::Ok(item) => Poll::Ready(item.take()),
            ReaderFut::Future { shared, idx } => {
                let guard = shared.inner.read().unwrap();
                match (guard.items.get(*idx), guard.closed) {
                    (Some(arc), _) => Poll::Ready(Some(arc.clone())),
                    (None, true) => Poll::Ready(None),
                    _ => {
                        drop(guard);
                        let mut guard = shared.inner.write().unwrap();
                        guard.wakers.push(cx.waker().clone());
                        Poll::Pending
                    }
                }
            }
        }
    }
}

impl<T> Shared<T> {
    pub fn new() -> Arc<Shared<T>> {
        Arc::new(Self {
            inner: RwLock::new(SharedInternal::new()),
        })
    }
    pub fn read(
        self: &Arc<Shared<T>>,
        idx: usize,
    ) -> impl Future<Output = <ReaderFut<T> as Future>::Output> {
        let inner = self.inner.read().unwrap();
        let items = &inner.items;
        match (items.get(idx), inner.closed) {
            (Some(arc), _) => ReaderFut::Ok(Some(arc.clone())),
            (None, true) => ReaderFut::Ok(None),
            _ => ReaderFut::Future {
                shared: self.clone(),
                idx,
            },
        }
    }

    pub fn write(self: &Arc<Shared<T>>, data: T) {
        let mut inner = self.inner.write().unwrap();
        inner.items.push(Arc::new(data));
        inner.wakers.iter().for_each(Waker::wake_by_ref);
        inner.wakers.clear();
    }

    pub fn close(self: &Arc<Shared<T>>) {
        self.inner.write().unwrap().closed = true;
    }
}

#[derive(Debug)]
pub struct LogWriter<T> {
    shared: Arc<Shared<T>>,
}

#[derive(Debug)]
pub struct LogReader<T> {
    shared: Arc<Shared<T>>,
    idx: usize,
}

#[derive(Debug)]
pub struct LogReaderFactory<T> {
    shared: Arc<Shared<T>>,
}

impl<T> LogReaderFactory<T> {
    pub fn create_reader(&self) -> LogReader<T> {
        LogReader {
            shared: self.shared.clone(),
            idx: 0,
        }
    }
}

impl<T> LogWriter<T> {
    pub fn write(&self, data: T) {
        self.shared.write(data)
    }
}

impl<T> Drop for LogWriter<T> {
    fn drop(&mut self) {
        self.shared.close();
    }
}

impl<T> LogReader<T> {
    fn read_next(&self, idx: &mut usize) -> impl Future<Output = <ReaderFut<T> as Future>::Output> {
        let fut = self.shared.read(*idx);
        *idx += 1;
        fut
    }
    pub fn into_stream(self) -> impl Stream<Item = Arc<T>> {
        async_stream::stream! {
            let mut i = self.idx;
            while let Some(arc) = self.read_next(&mut i).await {
                yield arc;
            }
        }
    }
}

pub fn log_channel<T>() -> (LogReaderFactory<T>, LogWriter<T>) {
    let shared = Shared::new();
    let factory = LogReaderFactory {
        shared: shared.clone(),
    };

    (factory, LogWriter { shared })
}

#[cfg(test)]
mod test {
    use tokio_stream::StreamExt;

    use super::*;

    #[tokio::test]
    async fn test_log() {
        let (factory, writer) = log_channel();

        let data = (1..=10).map(|i| format!("data{}", i)).collect::<Vec<_>>();
        let inner_data = data.clone();
        tokio::spawn(async move {
            for s in inner_data {
                writer.write(s);
            }
        });

        let s = factory.create_reader().into_stream();
        let res = s
            .collect::<Vec<_>>()
            .await
            .into_iter()
            .map(|s| s.as_ref().clone())
            .collect::<Vec<_>>();
        assert_eq!(res, data);

        let s2 = factory.create_reader().into_stream();
        let res = s2
            .collect::<Vec<_>>()
            .await
            .into_iter()
            .map(|s| s.as_ref().clone())
            .collect::<Vec<_>>();
        assert_eq!(res, data);
    }
}
