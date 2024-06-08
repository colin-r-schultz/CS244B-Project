use std::future::Future;
use std::sync::{Arc, Mutex};
use std::thread;

use tokio::sync::mpsc::{channel, Receiver, Sender};
use tokio::sync::{oneshot, Semaphore};

pub use crate::types::ResourceConfig;

pub trait Resources: Send + Sync {
    fn from_config(config: &self::ResourceConfig) -> Self;

    fn can_execute<'a>(
        mut reqs: impl Iterator<Item = (&'a str, usize)>,
        resources: &ResourceConfig,
    ) -> bool {
        reqs.all(|(res, amt)| resources.get(res).map(|&cap| amt <= cap).unwrap_or(false))
    }

    fn initialize(&self) -> impl Future<Output = ()> {
        async {}
    }
}

impl Resources for () {
    fn from_config(_config: &ResourceConfig) -> Self {}
}

type Thunk = Box<dyn FnOnce() -> () + Send>;

pub struct CpuResources {
    count: usize,
    tx: Sender<Thunk>,
}

impl Resources for CpuResources {
    fn from_config(config: &ResourceConfig) -> Self {
        let (tx, rx) = channel(1);
        let count = *config.get("cpus").unwrap_or(&0);
        let rx = Arc::new(Mutex::new(rx));
        for _ in 0..count {
            let rx = rx.clone();
            thread::spawn(move || thread_pool_task(rx));
        }
        Self { count, tx }
    }
}

fn thread_pool_task(rx: Arc<Mutex<Receiver<Thunk>>>) {
    while let Some(thunk) = rx.lock().ok().and_then(|mut rx| rx.blocking_recv()) {
        thunk();
    }
}

impl CpuResources {
    pub fn cpus<const N: usize>(&self) -> CpuHandle {
        assert!(N <= self.count);
        CpuHandle::new(N, self.tx.clone())
    }
}

pub struct CpuHandle {
    sema: Arc<Semaphore>,
    sender: Sender<Thunk>,
}

impl CpuHandle {
    fn new(n: usize, tx: Sender<Thunk>) -> Self {
        Self {
            sema: Arc::new(Semaphore::new(n)),
            sender: tx,
        }
    }

    pub async fn run<F: FnOnce() -> T + Send + 'static, T: Send + 'static>(
        &self,
        f: F,
    ) -> impl Future<Output = T> {
        let permit = self.sema.clone().acquire_owned().await;
        let (res_tx, res_rx) = oneshot::channel();
        self.sender
            .send(Box::new(move || {
                res_tx.send(f()).ok().unwrap();
                drop(permit);
            }))
            .await
            .ok()
            .unwrap();
        async move { res_rx.await.unwrap() }
    }
}
