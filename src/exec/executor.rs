use super::job::Job;
use serde_json::Value;

pub trait ExecutionContext {
    fn result(&mut self, job: &Job, is_async: bool) -> Option<Result<Option<Value>, Value>>;
}

pub trait Executor {
    type Context: ExecutionContext;

    fn execute(&mut self, job: &Job) -> Self::Context;
}