use serde::{Deserialize, Serialize};
use tokio::sync::oneshot;

use crate::dfut::DFutData;
use crate::types::{DFutId, InstanceId};

#[derive(Serialize, Deserialize)]
pub enum Command<CallType> {
    Call {
        id: DFutId,
        call: CallType,
    },
    Retrieve {
        data: DFutData,
        #[serde(skip)]
        channel: Option<oneshot::Sender<Box<[u8]>>>,
    },
    Completed {
        id: InstanceId,
        payload: Box<[u8]>,
    },
}
