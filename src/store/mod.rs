pub mod redis;

use crate::exec::node_type::NodeType;
use crate::exec::job_type::JobType;
use crate::exec::job::Job;
use crate::scheduler::ScheduleItem;
use uuid::Uuid;
use crate::exec::node::Node;
use serde_json::Value;

pub trait Store {
    fn connect() -> Result<Self, String> where Self: std::marker::Sized;
    fn replicate(&self) -> Result<StoreRef, String>;
    fn get_node_types(&mut self) -> Result<Vec<NodeType>, String>;
    fn set_node_type(&mut self, node_type_uuid: Uuid) -> Result<(), String>;
    fn get_job_types(&mut self) -> Result<Vec<JobType>, String>;
    // used when we encounter a job in our queue we don't know about
    fn get_new_job_type(&mut self, uuid: Uuid) -> Result<Option<JobType>, String>;
    fn get_job_schedule(&mut self) -> Result<Vec<ScheduleItem>, String>;
    fn claim_job_scheduled(&mut self, schedule_item: &ScheduleItem) -> Result<Option<ScheduleItem>, String>;
    fn enqueue_job(&mut self, job: Job) -> Result<(), String>;
    fn dequeue_job(&mut self) -> Result<Job, String>;
    fn finish_job(&mut self, job: Job, results: Option<Value>, errors: Option<Value>) -> Result<(), String>;
    fn ping(&mut self) -> Result<(), String>;
    fn get_ping_interval_ms(&self) -> u32;
    fn get_node(&mut self) -> &mut Node;
}

pub type StoreRef = Box<dyn Store + Send>;