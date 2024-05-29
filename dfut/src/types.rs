use uuid::Uuid;

pub type NodeId = u32;
pub type DFutId = Uuid;
pub type InstanceId = Uuid;

pub enum DFutError {
    Panic,
    Cancelled,
    Network,
}
