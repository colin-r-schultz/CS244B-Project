use std::any::Any;
use std::collections::HashMap;
use std::future::Future;
use std::io;
use std::net::SocketAddr;
use std::sync::OnceLock;

use serde::de::DeserializeOwned;
use tokio::net::TcpSocket;
use tokio::runtime::{Builder, Runtime};

use crate::connection::Connection;
use crate::dfut::{DFut, DFutData, DFutTrait, DFutValue};
use crate::store::{ObjectStore, PendingValue};
use crate::types::{DFutId, NodeId, Value};

pub struct Node<CallType: DFutTrait> {
    id: NodeId,
    addr_map: HashMap<NodeId, SocketAddr>,

    rt: Runtime,
    connections: HashMap<NodeId, Connection<CallType>>,

    store: ObjectStore,
}

impl<C: DFutTrait<CallType = C, Output = Value>> Node<C> {
    pub fn new(id: NodeId, addr_map: HashMap<NodeId, SocketAddr>) -> io::Result<Self> {
        let connections = addr_map
            .iter()
            .map(|(&conn_id, _)| (conn_id, Connection::new(conn_id)))
            .collect();
        Ok(Self {
            id,
            rt: Builder::new_current_thread().enable_io().build()?,
            addr_map,
            connections,
            store: ObjectStore::new(),
        })
    }

    pub fn start(self) {
        if let Err(_) = NODE.set(Box::new(self)) {
            panic!("Attempting to start second Node");
        }
        NODE.get().unwrap().downcast_ref::<Self>().unwrap().run();
    }

    fn run(&'static self) {
        self.connections.get(&self.id).unwrap().start_local(self);
        self.rt.block_on(async {
            let _handle = tokio::spawn(self.connect_remotes());
        });
    }

    async fn connect_remotes(&'static self) -> io::Result<()> {
        let listen_sock = TcpSocket::new_v4()?;
        listen_sock.set_reuseport(true)?;
        let myaddr = *self.addr_map.get(&self.id).unwrap();
        listen_sock.bind(myaddr)?;
        let listener = listen_sock.listen(self.addr_map.len() as u32)?;

        for (&id, &addr) in self.addr_map.iter() {
            if id != self.id {
                tokio::spawn(async move {
                    let sock = TcpSocket::new_v4()?;
                    sock.set_reuseport(true)?;
                    sock.bind(myaddr)?;
                    let stream = sock.connect(addr).await.unwrap();
                    self.connections
                        .get(&id)
                        .unwrap()
                        .start_remote(self, stream);
                    io::Result::Ok(())
                });
            }
        }

        loop {
            let (stream, new_addr) = match listener.accept().await {
                Ok(pair) => pair,
                Err(_) => continue,
            };
            if let Some((id, _)) = self.addr_map.iter().find(|(_, &addr)| addr == new_addr) {
                self.connections.get(id).unwrap().start_remote(self, stream);
            }
        }
    }

    fn spawn<T: DFutValue>(
        &self,
        call: impl DFutTrait<CallType = C, Output = T>,
    ) -> Result<DFut<C, T>, &'static str> {
        let conn = self.connections.values().nth(0).unwrap();
        conn.spawn(call)
    }

    pub(crate) fn run_task(&'static self, id: DFutId, call: C) {
        self.store.put(id, call.run(self))
    }

    pub(crate) fn get_from_store(&self, data: DFutData) -> PendingValue {
        self.store.get(data)
    }

    pub(crate) fn retrieve<T: Clone + DeserializeOwned + 'static>(
        &self,
        data: DFutData,
    ) -> impl Future<Output = T> + Send {
        self.connections.get(&data.node).unwrap().retrieve(data)
    }
}

static NODE: OnceLock<Box<dyn Sync + Send + Any>> = OnceLock::new();

pub fn spawn<T: DFutValue, C: DFutTrait<CallType = C, Output = Value>>(
    call: impl DFutTrait<CallType = C, Output = T>,
) -> Result<DFut<C, T>, &'static str> {
    NODE.get()
        .expect("Not in context")
        .downcast_ref::<Node<C>>()
        .unwrap()
        .spawn(call)
}
