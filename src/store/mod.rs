pub mod redis;

use crate::exec::node_type::NodeType;
use crate::exec::job_type::JobType;
use crate::exec::job::Job;
use crate::scheduler::ScheduleItem;
use uuid::Uuid;
use crate::exec::node::Node;
use serde_json::Value;
use log::*;
use crate::util::config;
use std::process::exit;

pub trait Store {
    fn connect() -> Result<Self, String> where Self: std::marker::Sized;
    fn replicate(&self) -> Result<StoreRef, String>;
    fn get_node_types(&mut self) -> Result<Vec<NodeType>, String>;
    fn get_node_type(&mut self, node_type_uuid: Uuid) -> Result<Option<NodeType>, String>;
    fn new_node_type(&mut self, node_type: &NodeType) -> Result<(), String>;
    fn set_node_type(&mut self, node_type_uuid: Uuid) -> Result<Option<()>, String>;
    fn set_node_type_soft(&mut self, node_type_uuid: Uuid) -> Result<Option<()>, String>; // used in some api calls to pretend to be other node types. does not update store.
    fn get_job_types(&mut self) -> Result<Vec<JobType>, String>;
    // used when we encounter a job in our queue we don't know about
    fn get_job_type(&mut self, uuid: Uuid) -> Result<Option<JobType>, String>;
    fn new_job_type(&mut self, job_type: &JobType) -> Result<(), String>;
    fn get_job_schedule(&mut self) -> Result<Vec<ScheduleItem>, String>;
    fn get_job_schedule_item(&mut self, uuid: Uuid) -> Result<Option<ScheduleItem>, String>;
    fn delete_job_schedule_item(&mut self, uuid: Uuid) -> Result<(), String>;
    fn new_job_schedule_item(&mut self, schedule_item: &ScheduleItem) -> Result<(), String>;
    fn claim_job_scheduled(&mut self, schedule_item: &ScheduleItem) -> Result<Option<ScheduleItem>, String>;
    fn enqueue_job(&mut self, job: Job) -> Result<(), String>;
    fn dequeue_job(&mut self) -> Result<Job, String>;
    fn get_all_jobs_waiting(&mut self) -> Result<Vec<Job>, String>;
    fn get_all_jobs_in_progress(&mut self) -> Result<Vec<Job>, String>;
    fn get_all_jobs_finished(&mut self) -> Result<Vec<Job>, String>;
    fn get_finished_job(&mut self, uuid: Uuid) -> Result<Option<Job>, String>;
    fn finish_job(&mut self, job: Job, results: Option<Value>, errors: Option<Value>) -> Result<(), String>;
    fn ping(&mut self) -> Result<(), String>;
    fn get_ping_interval_ms(&self) -> u32;
    fn get_node(&mut self) -> &mut Node;
    fn get_nodes(&mut self) -> Result<Vec<Node>, String>;
    fn get_other_node(&mut self, uuid: Uuid) -> Result<Option<Node>, String>;
    fn clean(&mut self);
}

pub type StoreRef = Box<dyn Store + Send>;

pub fn init_store_untyped() -> StoreRef {
    let store: StoreRef;
    if &*config::STORE_TYPE == "redis" {
        let redis_connected = redis::RedisStore::connect();
        match redis_connected {
            Err(e) => {
                error!("Error connecting to redis: {}", e);
                exit(1);
            }
            Ok(redis_store) => {
                store = Box::new(redis_store);
            }
        }
    } else {
        error!("Invalid STORE_TYPE configuration option: {}", &*config::STORE_TYPE);
        exit(1);
    }
    return store;
}

pub fn init_store() -> StoreRef {
    let mut store = init_store_untyped();
    let node_types = store.get_node_types();
    if node_types.is_err() {
        error!("Failed to get node types from store: {}", node_types.err().unwrap());
        exit(1);
    }
    let our_node_type = node_types.as_ref().unwrap().iter().find(|item| item.name == *config::NODE_TYPE);
    if our_node_type.is_none() {
        error!("Invalid node type specified, not found: {}", *config::NODE_TYPE);
        exit(1);
    }
    let raw_node_type = our_node_type.unwrap();
    let updated_redis_result = store.set_node_type(raw_node_type.uuid);
    if updated_redis_result.is_err() {
        error!("Failed to update node_type for node in store: {}", updated_redis_result.err().unwrap());
        exit(1);
    }
    if updated_redis_result.unwrap().is_none() {
        error!("Node type UUID was not found: {}", raw_node_type.uuid);
        exit(1);
    }
    return store;
}

#[cfg(test)]
pub mod tests {
    use super::*;
    use std::collections::HashMap;

    pub fn make_node_type(store: &mut StoreRef) -> Result<NodeType, String> {
        let test_node_type = NodeType {
            name: "test_node_type".to_string(),
            uuid: Uuid::new_v4(),
            thread_count: 1,
        };
        store.new_node_type(&test_node_type)?;
        return Ok(test_node_type);
    }

    pub fn make_job_type(store: &mut StoreRef) -> Result<JobType, String> {
        let mut test_job_type = JobType {
            executor: "bash".to_string(),
            name: "test".to_string(),
            node_type: "default".to_string(),
            timeout: None,
            unique: false,
            uuid: Uuid::new_v4(),
            metadata: HashMap::new(),
        };
        test_job_type.metadata.insert("command".to_string(), Value::String("echo 'test'".to_string()));
        store.new_job_type(&test_job_type)?;
        return Ok(test_job_type);
    }

    pub fn make_job(store: &mut StoreRef, job_type: &JobType) -> Result<Job, String> {
        let job = Job {
            uuid: Uuid::new_v4(),
            job_type_uuid: job_type.uuid,
            job_type: Some(job_type.clone()),
            arguments: HashMap::new(),
            executing_node: None,
            enqueued_at: None,
            started_at: None,
            ended_at: None,
            results: None,
            errors: None,
        };
        store.enqueue_job(job.clone())?;
        return Ok(job);
    }

    pub fn make_schedule_item(store: &mut StoreRef, job_type_uuid: Uuid, last_scheduled_by: Option<Uuid>, last_scheduled_at: Option<u64>) -> Result<ScheduleItem, String> {
        let mut test_schedule_item = ScheduleItem {
            uuid: Uuid::new_v4(),
            interval: 500,
            last_scheduled_by: last_scheduled_by,
            last_scheduled_at: last_scheduled_at,
            job_type_uuid: job_type_uuid,
            job_arguments: HashMap::new(),
        };
        test_schedule_item.job_arguments.insert("command".to_string(), Value::String("echo 'test'".to_string()));
        store.new_job_schedule_item(&test_schedule_item)?;
        return Ok(test_schedule_item);
    }

}