use serde_json::Value;
use std::collections::HashMap;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Deserialize, Serialize, Clone)]
pub struct ScheduleItem {
    pub uuid: Uuid,
    pub interval: u64,
    pub last_scheduled_by: Option<Uuid>,
    pub last_scheduled_at: Option<u64>,
    pub job_type_uuid: Uuid,
    pub job_arguments: HashMap<String, Value>,
}