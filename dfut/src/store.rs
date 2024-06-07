use std::collections::HashMap;
use std::future::Future;
use std::sync::Mutex;
use tokio::sync::broadcast::{self, Receiver, Sender};

use crate::dfut::DFutData;
use crate::types::{DFutId, InstanceId, Value};

pub struct TaskStore {
    map: Mutex<HashMap<DFutId, Entry>>,
}

impl TaskStore {
    pub fn new() -> Self {
        Self {
            map: Mutex::default(),
        }
    }

    pub fn put<F: Future<Output = Value> + Send + 'static>(&self, id: DFutId, task: F) {
        let tx = self
            .map
            .lock()
            .unwrap()
            .entry(id)
            .or_insert_with(Entry::new)
            .get_tx();
        tokio::spawn(async move {
            let _ = tx.send(task.await);
        });
    }

    pub fn get(&self, data: DFutData) -> PendingValue {
        let mut map = self.map.lock().unwrap();
        let entry = map.entry(data.id).or_insert_with(Entry::new);
        let done = entry.update_instances(&data);
        if !done {
            entry.get()
        } else {
            map.remove(&data.id).unwrap().take()
        }
    }
}

pub enum PendingValue {
    Pending(Receiver<Value>),
    Value(Value),
}

impl PendingValue {
    pub async fn resolve(self) -> Value {
        match self {
            Self::Pending(mut rx) => rx.recv().await.unwrap(),
            Self::Value(val) => val,
        }
    }
}

enum FutureValue {
    Pending(Sender<Value>, Receiver<Value>),
    Ready(Value),
}

struct Entry {
    value: FutureValue,
    instances: HashMap<InstanceId, i32>,
}

impl Entry {
    fn new() -> Self {
        let (tx, rx) = broadcast::channel(1);
        Self {
            value: FutureValue::Pending(tx, rx),
            instances: HashMap::from([(InstanceId::nil(), 1)]),
        }
    }

    fn get_tx(&self) -> Sender<Value> {
        match &self.value {
            FutureValue::Pending(tx, _) => tx.clone(),
            FutureValue::Ready(_) => unreachable!(),
        }
    }

    fn get(&mut self) -> PendingValue {
        match &mut self.value {
            FutureValue::Pending(tx, rx) => match rx.try_recv() {
                Ok(val) => {
                    self.value = FutureValue::Ready(val.clone());
                    PendingValue::Value(val)
                }
                Err(_) => return PendingValue::Pending(tx.subscribe()),
            },
            FutureValue::Ready(val) => PendingValue::Value(val.clone()),
        }
    }

    fn take(self) -> PendingValue {
        match self.value {
            FutureValue::Pending(_, mut rx) => match rx.try_recv() {
                Ok(val) => PendingValue::Value(val),
                Err(_) => PendingValue::Pending(rx),
            },
            FutureValue::Ready(val) => PendingValue::Value(val),
        }
    }

    fn update_instances(&mut self, data: &DFutData) -> bool {
        let parent_entry = self.instances.entry(data.parent).or_default();
        *parent_entry -= 1;
        if *parent_entry == 0 {
            self.instances.remove(&data.parent);
        }
        let curr_entry = self.instances.entry(data.instance_id).or_default();
        *curr_entry += data.children;
        assert!(*curr_entry >= 0);
        if *curr_entry == 0 {
            self.instances.remove(&data.parent);
        }

        self.instances.is_empty()
    }
}
