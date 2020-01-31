use std::process::*;
use crate::exec::executor::*;
use std::env;
use crate::exec::job::Job;
use log::*;
use serde_json::{Value, json};
use regex::Regex;
use serde_json::map::Map;

pub struct BashExecutor {

}

pub struct BashExecutorContext {
    internal_failure: bool,
    handle: Option<Child>,
}

impl ExecutionContext for BashExecutorContext {
    fn result(&mut self, job: &Job, is_async: bool, timeout: Option<u64>) -> Option<Result<Option<Value>, Value>> {
        //TODO: handle timeouts, async?
        if self.internal_failure {
            return Some(Err(Value::Null));
        }
        let handle = self.handle.as_mut().unwrap();
        let status = if is_async {
            let waited = handle.try_wait();
            if waited.is_ok() && waited.as_ref().unwrap().is_none() {
                return None;
            }
            waited.map(|i| i.unwrap())
        } else {
            handle.wait()
        };
        return match status {
            Err(e) => {
                error!("Failed to wait for bash process completion in job '{}', job type '{}' / '{}': {:?}", job.uuid.hyphenated(), job.job_type.as_ref().unwrap().name, job.job_type_uuid.hyphenated(), e);
                Some(Err(Value::Null))
            }
            Ok(status) => {
                let mut output = Map::new();
                let process_out = self.handle.take().unwrap().wait_with_output().expect("wait_with_output failed after wait succeeded");
                output.insert("stdout".to_string(), Value::String(String::from_utf8_lossy(process_out.stdout.as_slice()).to_string()));
                output.insert("stderr".to_string(), Value::String(String::from_utf8_lossy(process_out.stderr.as_slice()).to_string()));
                output.insert("exit_code".to_string(), status.code().map(|code| json!(code)).unwrap_or(Value::Null));
                Some(Ok(Some(Value::Object(output))))
            }
        }
    }
}

impl Executor for BashExecutor {
    type Context = BashExecutorContext;

    fn execute(&mut self, job: &Job) -> BashExecutorContext {
        lazy_static! {
            static ref SAFE_ARG_REGEX: Regex = Regex::new("\\\\|\"").unwrap();
        }
        let metadata = &job.job_type.as_ref().unwrap().metadata;
        let meta_command = metadata.get("command");
        let meta_environment = metadata.get("environment");
        if meta_command.is_none() || !(meta_command.unwrap().is_string() || meta_command.unwrap().is_array()) {
            error!("No command found in job type in bash execution for job '{}', job type '{}' / '{}'", job.uuid.hyphenated(), job.job_type.as_ref().unwrap().name, job.job_type_uuid.hyphenated());
            return BashExecutorContext { internal_failure: true, handle: None };
        }
        let command = job.arguments.get("command");
        let environment = job.arguments.get("environment");
        if command.is_some() && !(command.unwrap().is_string() || command.unwrap().is_array()) {
            error!("Invalid command arguments found in job in bash execution for job '{}', job type '{}' / '{}'", job.uuid.hyphenated(), job.job_type.as_ref().unwrap().name, job.job_type_uuid.hyphenated());
            return BashExecutorContext { internal_failure: true, handle: None };
        }
        let mut environment = match environment {
            None => Map::new(),
            Some(Value::Object(value)) => value.clone(),
            _ => {
                warn!("Invalid environment parameter in job for job '{}', job type '{}' / '{}'", job.uuid.hyphenated(), job.job_type.as_ref().unwrap().name, job.job_type_uuid.hyphenated());
                Map::new()
            }
        };
        let mut meta_environment = match meta_environment {
            None => Map::new(),
            Some(Value::Object(value)) => value.clone(),
            _ => {
                warn!("Invalid environment parameter in job type for job '{}', job type '{}' / '{}'", job.uuid.hyphenated(), job.job_type.as_ref().unwrap().name, job.job_type_uuid.hyphenated());
                Map::new()
            }
        };
        environment.append(&mut meta_environment);
        let empty_array = Value::Array(vec![]);
        let command_unwrapped = command.unwrap_or(&empty_array);
        let command_unwrapped_meta = meta_command.unwrap();
        let mut builder = Command::new(env::var("SHELL").unwrap_or("/bin/bash".to_string()));
        builder
            .stderr(Stdio::piped())
            .stdout(Stdio::piped())
            .arg("-c")
            .stdin(Stdio::null());
        environment.iter().for_each(|(key, value)| {
            if !value.is_string() {
                warn!("Non-string environment parameter '{}' for job '{}', job type '{}' / '{}'", key, job.uuid.hyphenated(), job.job_type.as_ref().unwrap().name, job.job_type_uuid.hyphenated());
            }
            builder.env(key, value.to_string());
        } );
        let mut total_command: Vec<String> = vec![];
        for unwrapped in vec![command_unwrapped, command_unwrapped_meta] {
            let arg = match unwrapped {
                Value::String(s) => s.clone(),
                Value::Array(arr) => {
                    if arr.iter().any({ |arg| !arg.is_string()}) {
                        error!("Invalid non-string argument in arguments for bash command for job '{}', job type '{}' / '{}'", job.uuid.hyphenated(), job.job_type.as_ref().unwrap().name, job.job_type_uuid.hyphenated());
                        return BashExecutorContext { internal_failure: true, handle: None };
                    }
                    arr.iter().map({ |arg| match arg {
                        Value::String(s) => format!("\"{}\"", &SAFE_ARG_REGEX.replace_all(s, "\\$0")),
                        _ => "".to_string(),
                    } }).collect::<Vec<String>>().join(" ")
                }
                _ => "".to_string()
            };
            total_command.push(arg);
        }
        builder.arg(total_command.join(" "));
        let handle = builder.spawn();
        if handle.is_err() {
            error!("Failed to spawn bash in bash execution for job '{}', job type '{}' / '{}': {:?}", job.uuid.hyphenated(), job.job_type.as_ref().unwrap().name, job.job_type_uuid.hyphenated(), handle.unwrap_err());
            return BashExecutorContext { internal_failure: true, handle: None };
        }
        return BashExecutorContext { internal_failure: false, handle: handle.ok() }
    }
}