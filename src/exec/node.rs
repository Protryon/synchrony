use uuid::Uuid;
use serde::{Deserialize, Serialize};
use super::node_type::NodeType;

#[derive(Deserialize, Serialize, Debug, Clone, PartialEq)]
pub struct Node {
    pub uuid: Uuid,
    pub node_type_uuid: Option<Uuid>,
    #[serde(skip)] pub node_type: Option<NodeType>,
    pub last_ping: u64,
}