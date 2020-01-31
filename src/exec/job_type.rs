use uuid::Uuid;
use serde_json::Value;
use serde::Deserialize;
use std::collections::HashMap;

#[derive(Deserialize, Debug, Clone)]
pub struct JobType {
    pub uuid: Uuid,
    pub name: String,
    pub executor: String,
    pub metadata: HashMap<String, Value>,
    pub unique: bool,
    pub node_type: String, // name not UUID to avoid versioning issues until node_types have more attached data
    pub timeout: Option<u64>,
}