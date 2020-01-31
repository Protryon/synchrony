use uuid::Uuid;
use serde::{Deserialize, Serialize};

#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct Node {
    pub uuid: Uuid,
    pub node_type_uuid: Option<Uuid>,
    pub last_ping: u64,
}