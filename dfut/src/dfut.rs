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
pub struct DFut<C: DFutTrait, T> {
    data: RefCell<DFutData>,
    node: &'static Node<C>,
    _marker: PhantomData<T>,
}

impl<C: DFutTrait, T> DFut<C, T> {
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

impl<C: DFutTrait, T> Clone for DFut<C, T> {
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

impl<C: DFutTrait, T> Into<DFutData> for DFut<C, T> {
    fn into(self) -> DFutData {
        self.data.into_inner()
    }
}

impl<C: DFutTrait, T: Clone + DeserializeOwned + 'static> IntoFuture for DFut<C, T> {
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

impl<T: Clone + DeserializeOwned + 'static> MaybeFut<T> {}

impl<T> From<T> for MaybeFut<T> {
    fn from(value: T) -> Self {
        Self::Val(value)
    }
}

impl<C: DFutTrait, T> From<DFut<C, T>> for MaybeFut<T> {
    fn from(value: DFut<C, T>) -> Self {
        Self::Fut(value.into())
    }
}

pub trait DFutTrait:
    DFutCall<Self, Output = Value> + Serialize + DeserializeOwned + Send + Sync + 'static
{
}

pub trait DFutCall<C: DFutTrait>: Into<C> + Sized {
    type Output: DFutValue;

    fn run(self, node: &'static Node<C>) -> impl Future<Output = Self::Output> + Send + 'static;

    fn to_call_type(self) -> C {
        self.into()
    }

    fn get_dfut_deps(&self) -> impl Iterator<Item = (NodeId, DFutId)>;
}

pub trait DFutValue: Any + erased_serde::Serialize + Send + Sync {}
impl<T> DFutValue for T where T: Any + Serialize + Send + Sync {}
erased_serde::serialize_trait_object!(DFutValue);

pub trait MaybeFutTrait<T>: Into<MaybeFut<T>> + Send + 'static {
    fn get_remote_dep(&self) -> Option<(NodeId, DFutId)>;

    fn retrieve<C: DFutTrait>(
        self,
        node: &'static Node<C>,
    ) -> impl std::future::Future<Output = T> + Send + 'static;
}

impl<T: Clone + DeserializeOwned + Send + 'static> MaybeFutTrait<T> for MaybeFut<T> {
    fn get_remote_dep(&self) -> Option<(NodeId, DFutId)> {
        match self {
            MaybeFut::Fut(f) => Some((f.node, f.id)),
            MaybeFut::Val(_) => None,
        }
    }

    fn retrieve<C: DFutTrait>(
        self,
        node: &'static Node<C>,
    ) -> impl std::future::Future<Output = T> + Send + 'static {
        async {
            match self {
                Self::Val(x) => x,
                Self::Fut(data) => node.retrieve(data).await,
            }
        }
    }
}

pub trait Resolve<T>: MaybeFutTrait<T> {
    fn resolve(self) -> impl Future<Output = T> + Send + 'static;
}

impl<T: Into<MaybeFut<T>> + Send + 'static> MaybeFutTrait<T> for T {
    fn get_remote_dep(&self) -> Option<(NodeId, DFutId)> {
        None
    }

    async fn retrieve<C: DFutTrait>(self, _node: &'static Node<C>) -> T {
        self
    }
}

impl<T: MaybeFutTrait<T>> Resolve<T> for T {
    async fn resolve(self) -> T {
        self
    }
}

impl<C: DFutTrait, T: Clone + DeserializeOwned + Send + 'static> MaybeFutTrait<T> for DFut<C, T> {
    fn get_remote_dep(&self) -> Option<(NodeId, DFutId)> {
        let data = self.data.borrow();
        Some((data.node, data.id))
    }

    fn retrieve<C2: DFutTrait>(
        self,
        _node: &'static Node<C2>,
    ) -> impl std::future::Future<Output = T> + Send + 'static {
        IntoFuture::into_future(self)
    }
}

impl<C: DFutTrait, T: Clone + DeserializeOwned + Send + 'static> Resolve<T> for DFut<C, T> {
    async fn resolve(self) -> T {
        self.await
    }
}
