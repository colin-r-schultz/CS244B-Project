use std::collections::HashMap;

use tokio::task::JoinHandle;

use crate::dfut::{DFutData, DFutValue};
use crate::types::{DFutId, InstanceId};

type Value = Box<dyn DFutValue>;

enum Retrieval<'a> {
    Borrowed(&'a dyn DFutValue),
    Owned(Value),
}

struct ObjectStore {
    map: HashMap<DFutId, Entry>,
}

impl ObjectStore {
    pub fn new() -> Self {
        todo!()
    }

    pub fn put(&self, id: DFutId, task: JoinHandle<Value>) {}

    pub async fn get(&self, key: DFutData) -> Retrieval {
        todo!()
    }
}

enum FutureValue {
    Pending(JoinHandle<Value>),
    Ready(Value),
}

struct Entry {
    value: FutureValue,
    requestors: HashMap<InstanceId, Option<usize>>,
}

impl Entry {
    pub async fn get(&self, key: DFutData) -> Retrieval {
        todo!()
    }
}
