use crate::types::{DFutId, InstanceId, NodeId, Value};
use crate::Node;
use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};
use std::any::Any;
use std::cell::RefCell;
use std::future::Future;
use std::marker::PhantomData;

#[derive(Serialize, Deserialize)]
pub struct DFutData {
    pub node: NodeId,
    pub id: DFutId,
    pub instance_id: InstanceId,
    pub parent: InstanceId,
    pub children: i32,
}

#[must_use]
pub struct DFut<T> {
    data: RefCell<DFutData>,
    _marker: PhantomData<T>,
}

impl<T> DFut<T> {
    pub fn new(node: NodeId, id: DFutId) -> Self {
        Self {
            data: RefCell::new(DFutData {
                node,
                id,
                instance_id: InstanceId::new_v4(),
                parent: InstanceId::nil(),
                children: 0,
            }),
            _marker: PhantomData,
        }
    }
}

impl<T> Clone for DFut<T> {
    fn clone(&self) -> Self {
        let mut data = self.data.borrow_mut();
        data.children += 1;
        let &DFutData {
            node,
            id,
            instance_id: parent,
            ..
        } = &*data;
        Self {
            data: RefCell::new(DFutData {
                node,
                id,
                instance_id: InstanceId::new_v4(),
                parent,
                children: 0,
            }),
            _marker: PhantomData,
        }
    }
}

impl<T> Into<DFutData> for DFut<T> {
    fn into(self) -> DFutData {
        self.data.into_inner()
    }
}

#[derive(Serialize, Deserialize)]
pub enum MaybeFut<T> {
    Val(T),
    Fut(DFutData),
}

impl<T: Clone + DeserializeOwned + 'static> MaybeFut<T> {
    pub fn add_dfut_dep(&self, deps: &mut Vec<(NodeId, DFutId)>) {
        if let Self::Fut(f) = self {
            deps.push((f.node, f.id));
        }
    }

    pub async fn resolve<C: DFutTrait<CallType = C, Output = Value>>(self, node: &Node<C>) -> T {
        match self {
            Self::Val(x) => x,
            Self::Fut(data) => node.retrieve(data).await,
        }
    }
}

impl<T> From<T> for MaybeFut<T> {
    fn from(value: T) -> Self {
        Self::Val(value)
    }
}

impl<T> From<DFut<T>> for MaybeFut<T> {
    fn from(value: DFut<T>) -> Self {
        Self::Fut(value.into())
    }
}

pub trait DFutTrait: Sized + Serialize + DeserializeOwned + Send + 'static {
    type Output: DFutValue;
    type CallType: From<Self> + DFutTrait;

    fn run(
        self,
        node: &'static Node<Self::CallType>,
    ) -> impl Future<Output = Self::Output> + Send + 'static;

    fn to_call_type(self) -> Self::CallType {
        self.into()
    }

    fn get_dfut_deps(&self) -> Vec<(NodeId, DFutId)>;
}

pub trait DFutValue: Any + erased_serde::Serialize + Send + Sync {}
impl<T> DFutValue for T where T: Any + Serialize + Send + Sync {}
erased_serde::serialize_trait_object!(DFutValue);