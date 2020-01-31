use uuid::Uuid;
use serde::Deserialize;

#[derive(Deserialize, Debug)]
pub struct NodeType {
    pub uuid: Uuid,
    pub name: String,
    pub thread_count: u32,
}