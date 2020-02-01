use uuid::Uuid;
use serde::{ Deserialize, Serialize };

#[derive(Deserialize, Serialize, Debug, PartialEq, Clone)]
pub struct NodeType {
    pub uuid: Uuid,
    pub name: String,
    pub thread_count: u32,
}