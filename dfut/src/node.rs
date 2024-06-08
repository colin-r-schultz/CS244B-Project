use std::any::Any;
use std::collections::HashMap;
use std::future::Future;
use std::io;
use std::net::SocketAddr;
use std::sync::OnceLock;

use rand::seq::IteratorRandom;
use rand::thread_rng;
use serde::de::DeserializeOwned;
use tokio::net::TcpSocket;
use tokio::runtime::{Builder, Runtime};
use tokio::task::{JoinHandle, JoinSet};

use crate::connection::Connection;
use crate::dfut::{DFut, DFutCall, DFutData, DFutTrait, DFutValue};
use crate::resource::Resources;
use crate::store::{PendingValue, TaskStore};
use crate::types::{DFutId, NodeId, ResourceConfig};

pub struct Node<CallType: DFutTrait> {
    id: NodeId,
    addr_map: HashMap<NodeId, SocketAddr>,

    rt: Runtime,
    connections: HashMap<NodeId, Connection<CallType>>,

    resources: CallType::Resources,
    store: TaskStore,
}

impl<C: DFutTrait> Node<C> {
    pub fn new(
        id: NodeId,
        config: HashMap<NodeId, (SocketAddr, ResourceConfig)>,
    ) -> io::Result<Self> {
        let resources = C::Resources::from_config(&config.get(&id).unwrap().1);
        let mut connections = HashMap::new();
        let mut addr_map = HashMap::new();
        for (conn_id, (addr, resources)) in config.into_iter() {
            connections.insert(conn_id, Connection::new(conn_id, resources));
            addr_map.insert(conn_id, addr);
        }
        Ok(Self {
            id,
            rt: Builder::new_current_thread().enable_all().build()?,
            addr_map,
            connections,
            store: TaskStore::new(),
            resources,
        })
    }

    pub fn start(self, main: Option<impl DFutCall<C, Output = ()>>) {
        if let Err(_) = NODE.set(Box::new(self)) {
            panic!("Attempting to start second Node");
        }
        NODE.get()
            .unwrap()
            .downcast_ref::<Self>()
            .unwrap()
            .run(main);
    }

    fn run(&'static self, main: Option<impl DFutCall<C, Output = ()>>) {
        self.connections.get(&self.id).unwrap().start_local(self);
        self.rt.block_on(async {
            let (listen_task, mut connects) = self.listen_for_remotes();
            self.resources.initialize().await;
            let mut connect_results = Vec::new();
            while let Some(res) = connects.join_next().await {
                connect_results.push(res.unwrap());
            }
            if let Some(main) = main {
                // connect_results
                //     .into_iter()
                //     .for_each(|x| x.expect("Leader failed to connect"));
                self.connections
                    .get(&self.id)
                    .unwrap()
                    .spawn(main)
                    .ok()
                    .unwrap()
                    .await
            } else {
                listen_task.await.unwrap();
            }
        });
    }

    fn listen_for_remotes(&'static self) -> (JoinHandle<()>, JoinSet<io::Result<()>>) {
        let listen_sock = TcpSocket::new_v4().unwrap();
        listen_sock.set_reuseport(true).unwrap();
        let myaddr = *self.addr_map.get(&self.id).unwrap();
        listen_sock.bind(myaddr).unwrap();
        let listener = listen_sock.listen(self.addr_map.len() as u32).unwrap();

        let mut set = JoinSet::new();

        for (&id, &addr) in self.addr_map.iter() {
            if id != self.id {
                set.spawn(async move {
                    let sock = TcpSocket::new_v4()?;
                    sock.set_reuseport(true)?;
                    sock.bind(myaddr)?;
                    let stream = sock.connect(addr).await?;
                    self.connections
                        .get(&id)
                        .unwrap()
                        .start_remote(self, stream);
                    io::Result::Ok(())
                });
            }
        }

        let listen_task = tokio::spawn(async move {
            loop {
                let (stream, new_addr) = match listener.accept().await {
                    Ok(pair) => pair,
                    Err(_) => continue,
                };
                if let Some((id, _)) = self.addr_map.iter().find(|(_, &addr)| addr == new_addr) {
                    self.connections.get(id).unwrap().start_remote(self, stream);
                }
            }
        });
        (listen_task, set)
    }

    pub fn resources(&self) -> &C::Resources {
        &self.resources
    }

    fn spawn<T: DFutValue>(&self, call: impl DFutCall<C, Output = T>) -> DFut<C, T> {
        self.connections
            .iter()
            .filter(|(_, conn)| conn.can_execute(&call))
            .choose(&mut thread_rng())
            .unwrap()
            .1
            .spawn(call)
            .ok()
            .unwrap()
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

impl<C: DFutTrait> Node<C> {
    pub(crate) fn run_task(&'static self, id: DFutId, call: C) {
        self.store.put(id, call.run(self))
    }
}

static NODE: OnceLock<Box<dyn Sync + Send + Any>> = OnceLock::new();

pub fn spawn<T: DFutValue, C: DFutTrait>(call: impl DFutCall<C, Output = T>) -> DFut<C, T> {
    NODE.get()
        .expect("Not in context")
        .downcast_ref::<Node<C>>()
        .unwrap()
        .spawn(call)
}
