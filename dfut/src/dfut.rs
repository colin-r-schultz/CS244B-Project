use crate::types::{DFutId, InstanceId, NodeId, Value};
use crate::Node;
use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};
use std::any::Any;
use std::cell::RefCell;
use std::future::{Future, IntoFuture};
use std::marker::PhantomData;
use std::pin::Pin;

#[derive(Serialize, Deserialize)]
pub struct DFutData {
    pub node: NodeId,
    pub id: DFutId,
    pub instance_id: InstanceId,
    pub parent: InstanceId,
    pub children: i32,
}

#[must_use]
pub struct DFut<C: DFutTrait<CallType = C, Output = Value>, T> {
    data: RefCell<DFutData>,
    node: &'static Node<C>,
    _marker: PhantomData<T>,
}

impl<C: DFutTrait<CallType = C, Output = Value>, T> DFut<C, T> {
    pub fn new(node: &'static Node<C>, node_id: NodeId, id: DFutId) -> Self {
        Self {
            data: RefCell::new(DFutData {
                node: node_id,
                id,
                instance_id: InstanceId::new_v4(),
                parent: InstanceId::nil(),
                children: 0,
            }),
            node,
            _marker: PhantomData,
        }
    }
}

impl<C: DFutTrait<CallType = C, Output = Value>, T> Clone for DFut<C, T> {
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
            node: self.node,
            _marker: PhantomData,
        }
    }
}

impl<C: DFutTrait<CallType = C, Output = Value>, T> Into<DFutData> for DFut<C, T> {
    fn into(self) -> DFutData {
        self.data.into_inner()
    }
}

impl<C: DFutTrait<CallType = C, Output = Value>, T: Clone + DeserializeOwned + 'static> IntoFuture
    for DFut<C, T>
{
    type Output = T;

    type IntoFuture = Pin<Box<dyn Future<Output = Self::Output> + Send>>;

    fn into_future(self) -> Self::IntoFuture {
        Box::pin(self.node.retrieve(self.into()))
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

impl<C: DFutTrait<CallType = C, Output = Value>, T> From<DFut<C, T>> for MaybeFut<T> {
    fn from(value: DFut<C, T>) -> Self {
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
