use std::collections::HashMap;
use serde::{Deserialize, Serialize};
use uuid::Uuid;
use serde_json::Value;
use super::job_type::JobType;

#[derive(Deserialize, Serialize, Debug, PartialEq, Clone)]
pub struct Job {
    pub uuid: Uuid,
    pub job_type_uuid: Uuid,
    #[serde(skip)] pub job_type: Option<JobType>,
    pub arguments: HashMap<String, Value>,
    pub executing_node: Option<Uuid>,
    pub enqueued_at: Option<u64>,
    pub started_at: Option<u64>,
    pub ended_at: Option<u64>,
    pub results: Option<Value>,
    pub errors: Option<Value>,
}
