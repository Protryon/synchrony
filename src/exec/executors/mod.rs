pub mod bash;
pub mod sidekiq;

use super::job::Job;
use crate::store::StoreRef;
use super::executor::*;
use log::*;
use serde_json::Value;

fn finish_job_execution(store: &mut StoreRef, job: Job, result: Option<Result<Option<Value>, Value>>) {
    let finish_result = match result {
        Some(Err(e)) => {
            store.finish_job(job, None, Some(e))
        },
        Some(Ok(value)) => {
            store.finish_job(job, value, None)
        },
        _ => {
            store.finish_job(job, None, Some(Value::String("invalid executor context [async not supported]".to_string())))
        },
    };
    if finish_result.is_err() {
        error!("Error finishing bash task from redis server: {}", finish_result.err().unwrap());
        return;
    }
}

pub fn run_job(store: &mut StoreRef, job: Job) {
    let job_type = job.job_type.as_ref().unwrap();
    if job_type.executor == "bash" {
        let mut executor = bash::BashExecutor {};
        let mut context = executor.execute(&job);
        let result = context.result(&job, false);
        finish_job_execution(store, job, result);
    } else if job_type.executor == "sidekiq" {
        let mut executor = sidekiq::SidekiqExecutor {};
        let mut context = executor.execute(&job);
        let result = context.result(&job, false);
        finish_job_execution(store, job, result);
    } else {
        error!("Invalid executor type for job type '{}' / '{}' on job '{}': '{}'", job_type.name, job_type.uuid.hyphenated(), job.uuid.hyphenated(), job_type.executor);
        let finish_result = store.finish_job(job, None, None);
        if finish_result.is_err() {
            error!("Error finishing bash task from redis server: {}", finish_result.err().unwrap());
            return;
        }
    }
}