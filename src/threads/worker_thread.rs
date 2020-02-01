
use std::thread;
use crate::StoreRef;
use std::sync::{ Mutex, Arc };
use log::*;
use crate::exec::executors::*;
use crate::exec::executor::*;
use serde_json::Value;

pub fn start_thread(store: Arc<Mutex<StoreRef>>) {
    thread::spawn(move || {
        loop {
            let mut guarded_store = store.lock().unwrap();
            let dequeued_item = guarded_store.dequeue_job();
            if dequeued_item.is_err() {
                error!("Error getting job schedule from redis server: {}", dequeued_item.err().unwrap());
                continue;
            }
            let job = dequeued_item.unwrap();
            let job_type = job.job_type.as_ref().unwrap();
            info!("Starting job '{}' of type '{}' / '{}'", job.uuid.hyphenated(), job_type.name, job.job_type_uuid.hyphenated());
            if job_type.executor == "bash" {
                let mut executor = bash::BashExecutor {};
                let mut context = executor.execute(&job);
                let result = context.result(&job, false);
                let finish_result = match result {
                    Some(Err(e)) => {
                        guarded_store.finish_job(job, None, Some(e))
                    },
                    Some(Ok(value)) => {
                        guarded_store.finish_job(job, value, None)
                    },
                    _ => {
                        guarded_store.finish_job(job, None, Some(Value::String("invalid executor context [async not supported]".to_string())))
                    },
                };
                if finish_result.is_err() {
                    error!("Error finishing bash task from redis server: {}", finish_result.err().unwrap());
                    continue;
                }
            } else {
                error!("Invalid executor type for job type '{}' / '{}' on job '{}': '{}'", job_type.name, job_type.uuid.hyphenated(), job.uuid.hyphenated(), job_type.executor);
                let finish_result = guarded_store.finish_job(job, None, None);
                if finish_result.is_err() {
                    error!("Error finishing bash task from redis server: {}", finish_result.err().unwrap());
                    continue;
                }
            }
        }
    });
}
