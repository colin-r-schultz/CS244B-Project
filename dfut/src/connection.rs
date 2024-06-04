use serde::de::DeserializeOwned;
use std::any::TypeId;
use std::collections::HashMap;
use std::future::Future;
use std::io::{self, ErrorKind};
use std::pin::Pin;
use std::sync::{Arc, Mutex};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpStream;
use tokio::sync::mpsc::{self, UnboundedReceiver as Receiver, UnboundedSender as Sender};
use tokio::sync::oneshot;
use tokio::task::JoinSet;

use crate::dfut::{DFut, DFutCall, DFutData, DFutTrait, DFutValue};
use crate::protocol::Command;
use crate::types::{DFutId, InstanceId, NodeId, Value};
use crate::Node;

pub struct Connection<C: DFutTrait> {
    id: NodeId,
    session: Mutex<Option<Session<C>>>,
}

impl<C: DFutTrait> Connection<C> {
    pub fn new(id: NodeId) -> Self {
        Self {
            id,
            session: Mutex::default(),
        }
    }

    pub fn start_local(&self, node: &'static Node<C>) {
        let old = std::mem::replace(
            &mut *self.session.lock().unwrap(),
            Some(Session::new_local(node, self.id)),
        );
        assert!(matches!(old, None));
    }

    pub fn start_remote(&self, node: &'static Node<C>, stream: TcpStream) {
        std::mem::replace(
            &mut *self.session.lock().unwrap(),
            Some(Session::new_remote(node, self.id, stream)),
        )
        .map(Session::abort);
    }

    pub fn spawn<T: DFutValue, A: DFutCall<C, Output = T>>(
        &self,
        call: A,
    ) -> Result<DFut<C, T>, A> {
        if let Some(sess) = &*self.session.lock().unwrap() {
            sess.spawn(call)
        } else {
            Err(call)
        }
    }

    pub fn retrieve<T: Clone + DeserializeOwned + 'static>(
        &self,
        data: DFutData,
    ) -> impl Future<Output = T> + Send {
        if let Some(sess) = &*self.session.lock().unwrap() {
            sess.retrieve(data)
        } else {
            panic!("No session");
        }
    }
}

enum SessionType<C> {
    Local,
    Remote {
        tasks: JoinSet<()>,
        call_channel: Sender<Command<C>>,
    },
}

struct Session<C: DFutTrait> {
    node: &'static Node<C>,
    connected_id: NodeId,
    session_type: SessionType<C>,
}

impl<C: DFutTrait> Session<C> {
    fn new_local(node: &'static Node<C>, connected_id: NodeId) -> Self {
        Self {
            node,
            connected_id,
            session_type: SessionType::Local,
        }
    }

    fn new_remote(node: &'static Node<C>, connected_id: NodeId, stream: TcpStream) -> Self {
        let (sender, receiver) = mpsc::unbounded_channel();
        let sender_clone = sender.clone();
        let mut tasks = JoinSet::new();
        tasks.spawn(async move {
            Self::task(SessionState {
                node,
                stream,
                sender,
                receiver,
                outstanding_requests: HashMap::new(),
            })
            .await
            .unwrap()
        });
        Self {
            node,
            connected_id,
            session_type: SessionType::Remote {
                call_channel: sender_clone,
                tasks,
            },
        }
    }

    fn spawn<T, A: DFutCall<C, Output = T>>(&self, call: A) -> Result<DFut<C, T>, A> {
        let id = DFutId::new_v4();
        match &self.session_type {
            SessionType::Local => self.node.run_task(id, call.to_call_type()),
            SessionType::Remote { call_channel, .. } => {
                if call_channel.is_closed() {
                    return Err(call);
                } else {
                    call_channel
                        .send(Command::Call {
                            id,
                            call: call.to_call_type(),
                        })
                        .unwrap()
                }
            }
        };
        Ok(DFut::new(self.node, self.connected_id, id))
    }

    pub fn retrieve<T: Clone + DeserializeOwned + 'static>(
        &self,
        data: DFutData,
    ) -> impl Future<Output = T> + Send {
        match &self.session_type {
            SessionType::Local => {
                let pending = self.node.get_from_store(data);
                Box::pin(async { Arc::unwrap_or_clone(cast(pending.resolve().await).unwrap()) })
                    as Pin<Box<dyn Future<Output = T> + Send>>
            }
            SessionType::Remote { call_channel, .. } => {
                let (tx, rx) = oneshot::channel();
                call_channel
                    .send(Command::Retrieve {
                        data,
                        channel: Some(tx),
                    })
                    .unwrap();
                Box::pin(async { serde_cbor::from_slice(&rx.await.unwrap()).unwrap() })
            }
        }
    }

    fn abort(self) {
        match self.session_type {
            SessionType::Local { .. } => panic!("Attempting to abort local session"),
            SessionType::Remote { mut tasks, .. } => tasks.abort_all(),
        }
    }

    async fn task(mut state: SessionState<C>) -> io::Result<()> {
        loop {
            tokio::select! {
                cmd = state.receiver.recv() => match cmd {
                    Some(cmd) => Self::send_cmd(&mut state, cmd).await?,
                    None => break,
                },

                res = state.stream.readable() => res.map(|_| Self::recv_cmd(&mut state))?.await?,
            };
        }
        Ok(())
    }

    async fn send_cmd(state: &mut SessionState<C>, mut cmd: Command<C>) -> io::Result<()> {
        if let Command::Retrieve { data, channel } = &mut cmd {
            state
                .outstanding_requests
                .insert(data.instance_id, channel.take().unwrap());
        }
        let payload = serde_cbor::to_vec(&cmd).unwrap();
        state
            .stream
            .write_u32(payload.len().try_into().unwrap())
            .await?;
        state.stream.write_all(&payload).await
    }

    async fn recv_cmd(state: &mut SessionState<C>) -> io::Result<()> {
        let mut len_buf = [0; 4];
        match state.stream.try_read(&mut len_buf) {
            Ok(n) => {
                state.stream.read_exact(&mut len_buf[n..]).await?;
            }
            Err(e) if e.kind() == ErrorKind::WouldBlock => return Ok(()),
            Err(e) => return Err(e),
        }

        let len = u32::from_be_bytes(len_buf);
        let mut buf = vec![0; len as usize];

        state.stream.read_exact(&mut buf).await?;
        let cmd: Command<C> = serde_cbor::from_slice(&buf).unwrap();
        match cmd {
            Command::Call { id, call } => state.node.run_task(id, call),
            Command::Retrieve { data, .. } => {
                let sender = state.sender.clone();
                let node = state.node;
                tokio::spawn(async move {
                    let id = data.instance_id;
                    let val = node.get_from_store(data).resolve().await;
                    let payload = serde_cbor::to_vec(&val).unwrap().into_boxed_slice();
                    sender.send(Command::Completed { id, payload }).unwrap();
                });
            }
            Command::Completed { id, payload } => state
                .outstanding_requests
                .remove(&id)
                .unwrap()
                .send(payload)
                .unwrap(),
        };
        Ok(())
    }
}

struct SessionState<C: DFutTrait> {
    node: &'static Node<C>,
    stream: TcpStream,
    sender: Sender<Command<C>>,
    receiver: Receiver<Command<C>>,
    outstanding_requests: HashMap<InstanceId, oneshot::Sender<Box<[u8]>>>,
}

fn cast<T: 'static>(val: Value) -> Option<Arc<T>> {
    (val.as_ref().type_id() == TypeId::of::<T>())
        .then(|| unsafe { Arc::from_raw(Arc::into_raw(val).cast()) })
}
