use std::io::{self};
use std::sync::Mutex;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpStream;
use tokio::sync::mpsc::{self, Receiver, Sender};
use tokio::task::JoinSet;

use crate::dfut::{DFut, DFutTrait, DFutValue};
use crate::protocol::Command;
use crate::types::{DFutId, NodeId};
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

    pub fn spawn<T: DFutValue>(
        &self,
        call: impl DFutTrait<CallType = C, Output = T>,
    ) -> Result<DFut<T>, &'static str> {
        if let Some(sess) = &*self.session.lock().unwrap() {
            sess.spawn(call)
        } else {
            Err("No current session")
        }
    }
}

enum SessionType<C: DFutTrait> {
    Local {
        node: &'static Node<C>,
    },
    Remote {
        tasks: JoinSet<()>,
        call_channel: Sender<Command<C>>,
    },
}

struct Session<C: DFutTrait> {
    connected_id: NodeId,
    session_type: SessionType<C>,
}

impl<C: DFutTrait> Session<C> {
    fn new_local(node: &'static Node<C>, connected_id: NodeId) -> Self {
        Self {
            connected_id,
            session_type: SessionType::Local { node },
        }
    }

    fn new_remote(node: &'static Node<C>, connected_id: NodeId, stream: TcpStream) -> Self {
        let (sender, receiver) = mpsc::channel(10);
        let mut tasks = JoinSet::new();
        tasks.spawn(async { Self::task(node, stream, receiver).await.unwrap() });
        Self {
            connected_id,
            session_type: SessionType::Remote {
                call_channel: sender,
                tasks,
            },
        }
    }

    fn spawn<T: Send>(
        &self,
        call: impl DFutTrait<CallType = C, Output = T>,
    ) -> Result<DFut<T>, &'static str> {
        let id = DFutId::new_v4();
        match &self.session_type {
            SessionType::Local { node } => node.run_task(id, call.to_call_type()),
            SessionType::Remote { call_channel, .. } => call_channel
                .try_send(Command::Call {
                    id,
                    call: call.to_call_type(),
                })
                .unwrap(),
        };
        Ok(DFut::new(self.connected_id, id))
    }

    fn abort(self) {
        match self.session_type {
            SessionType::Local { .. } => panic!("Attempting to abort local session"),
            SessionType::Remote { mut tasks, .. } => tasks.abort_all(),
        }
    }

    async fn task(
        node: &Node<C>,
        mut stream: TcpStream,
        mut send_channel: Receiver<Command<C>>,
    ) -> io::Result<()> {
        loop {
            tokio::select! {
                cmd = send_channel.recv() => match cmd {
                    Some(cmd) => Self::send_cmd(&mut stream, cmd).await?,
                    None => break,
                },

                res = stream.readable() => res.map(|_| Self::recv_cmd(node, &mut stream))?.await?,
            };
        }
        Ok(())
    }

    async fn send_cmd(stream: &mut TcpStream, cmd: Command<C>) -> io::Result<()> {
        let payload = serde_cbor::to_vec(&cmd).unwrap();
        stream.write_u32(payload.len().try_into().unwrap()).await?;
        stream.write_all(&payload).await
    }

    async fn recv_cmd(node: &Node<C>, stream: &mut TcpStream) -> io::Result<()> {
        let mut len_buf = [0; 4];

        if stream.peek(&mut len_buf).await? == 0 {
            return Ok(());
        }
        let len = stream.read_u32().await?;

        let mut buf = vec![0; len as usize];

        stream.read_exact(&mut buf).await?;
        let cmd: Command<C> = serde_cbor::from_slice(&buf).unwrap();
        match cmd {
            Command::Call { id, call } => node.run_task(id, call),
            _ => todo!(),
        };
        Ok(())
    }
}
