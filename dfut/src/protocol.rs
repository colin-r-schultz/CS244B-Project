use serde::{Deserialize, Serialize};

use crate::types::DFutId;

#[derive(Serialize, Deserialize)]
pub enum Command<CallType> {
    Call { id: DFutId, call: CallType },
    Retrieve { id: DFutId },
    Completed { payload: Box<[u8]> },
}
