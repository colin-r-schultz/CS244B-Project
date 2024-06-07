use std::collections::HashMap;
use std::sync::Arc;

use uuid::Uuid;

use crate::dfut::DFutValue;

pub type NodeId = u32;
pub type DFutId = Uuid;
pub type InstanceId = Uuid;

pub type Value = Arc<dyn DFutValue>;

pub type ResourceConfig = HashMap<String, usize>;
