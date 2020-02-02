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
    pub internal_failure: bool,
    pub handle: Option<Child>,
    pub timeout: Option<u64>,
}

impl ExecutionContext for BashExecutorContext {
    fn result(&mut self, job: &Job, is_async: bool) -> Option<Result<Option<Value>, Value>> {
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
        let timeout = job.job_type.as_ref().unwrap().timeout;
        if meta_command.is_none() || !(meta_command.unwrap().is_string() || meta_command.unwrap().is_array()) {
            error!("No command found in job type in bash execution for job '{}', job type '{}' / '{}'", job.uuid.hyphenated(), job.job_type.as_ref().unwrap().name, job.job_type_uuid.hyphenated());
            return BashExecutorContext { internal_failure: true, handle: None, timeout: timeout };
        }
        let command = job.arguments.get("command");
        let environment = job.arguments.get("environment");
        if command.is_some() && !(command.unwrap().is_string() || command.unwrap().is_array()) {
            error!("Invalid command arguments found in job in bash execution for job '{}', job type '{}' / '{}'", job.uuid.hyphenated(), job.job_type.as_ref().unwrap().name, job.job_type_uuid.hyphenated());
            return BashExecutorContext { internal_failure: true, handle: None, timeout: timeout };
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
        meta_environment.append(&mut environment);
        environment = meta_environment;
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
            match value {
                Value::String(s) => {
                    builder.env(key, s);
                }
                _ => {
                    warn!("Non-string environment parameter '{}' for job '{}', job type '{}' / '{}'", key, job.uuid.hyphenated(), job.job_type.as_ref().unwrap().name, job.job_type_uuid.hyphenated());
                }
            }            
        } );
        let mut total_command: Vec<String> = vec![];
        for unwrapped in vec![command_unwrapped_meta, command_unwrapped] {
            let arg = match unwrapped {
                Value::String(s) => s.clone(),
                Value::Array(arr) => {
                    if arr.iter().any({ |arg| !arg.is_string()}) {
                        error!("Invalid non-string argument in arguments for bash command for job '{}', job type '{}' / '{}'", job.uuid.hyphenated(), job.job_type.as_ref().unwrap().name, job.job_type_uuid.hyphenated());
                        return BashExecutorContext { internal_failure: true, handle: None, timeout: timeout };
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
            return BashExecutorContext { internal_failure: true, handle: None, timeout: timeout };
        }
        return BashExecutorContext { internal_failure: false, handle: handle.ok(), timeout: timeout }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use uuid::Uuid;
    use crate::exec::job_type::JobType;
    use std::collections::HashMap;
    use serde_json::Number;

    fn make_job_type(command: Value, environment: Option<Map<String, Value>>) -> JobType {
        let mut job_type = JobType {
            executor: "bash".to_string(),
            name: "test".to_string(),
            node_type: "default".to_string(),
            timeout: None,
            unique: false,
            uuid: Uuid::new_v4(),
            metadata: HashMap::new(),
        };
        job_type.metadata.insert("command".to_string(), command);
        if environment.is_some() {
            job_type.metadata.insert("environment".to_string(), Value::Object(environment.unwrap()));
        }
        return job_type;
    }

    fn make_job(job_type: &JobType, command: Option<Value>, environment: Option<Map<String, Value>>) -> Job {
        let mut job = Job {
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
        if command.is_some() {
            job.arguments.insert("command".to_string(), command.unwrap());
        }
        if environment.is_some() {
            job.arguments.insert("environment".to_string(), Value::Object(environment.unwrap()));
        }
        return job;
    }

    #[test]
    fn can_execute_command() {
        let mut executor = BashExecutor {};
        let job_type = make_job_type(Value::String("echo 'test'".to_string()), None);
        let job = make_job(&job_type, None, None);
        let mut context = executor.execute(&job);
        let result = context.result(&job, false);
        let mut output = Map::new();
        output.insert("stdout".to_string(), Value::String("test\n".to_string()));
        output.insert("stderr".to_string(), Value::String("".to_string()));
        output.insert("exit_code".to_string(), Value::Number(Number::from(0)));
        assert_eq!(result, Some(Ok(Some(Value::Object(output)))));
    }

    #[test]
    fn can_execute_commands() {
        let mut executor = BashExecutor {};
        let job_type = make_job_type(Value::String("echo 'test'; echo 'test2'; echo 'test_err' 1>&2; exit 1;".to_string()), None);
        let job = make_job(&job_type, None, None);
        let mut context = executor.execute(&job);
        let result = context.result(&job, false);
        let mut output = Map::new();
        output.insert("stdout".to_string(), Value::String("test\ntest2\n".to_string()));
        output.insert("stderr".to_string(), Value::String("test_err\n".to_string()));
        output.insert("exit_code".to_string(), Value::Number(Number::from(1)));
        assert_eq!(result, Some(Ok(Some(Value::Object(output)))));
    }

    #[test]
    fn can_execute_array_command() {
        let mut executor = BashExecutor {};
        let job_type = make_job_type(Value::Array(vec![Value::String("echo".to_string()), Value::String("test complex".to_string())]), None);
        let job = make_job(&job_type, None, None);
        let mut context = executor.execute(&job);
        let result = context.result(&job, false);
        let mut output = Map::new();
        output.insert("stdout".to_string(), Value::String("test complex\n".to_string()));
        output.insert("stderr".to_string(), Value::String("".to_string()));
        output.insert("exit_code".to_string(), Value::Number(Number::from(0)));
        assert_eq!(result, Some(Ok(Some(Value::Object(output)))));
    }

    #[test]
    fn can_execute_parameterized_command() {
        let mut executor = BashExecutor {};
        let job_type = make_job_type(Value::Array(vec![Value::String("echo".to_string()), Value::String("test complex".to_string())]), None);
        let job = make_job(&job_type, Some(Value::Array(vec![Value::String("part 2".to_string()), Value::String("part 3".to_string())])), None);
        let mut context = executor.execute(&job);
        let result = context.result(&job, false);
        let mut output = Map::new();
        output.insert("stdout".to_string(), Value::String("test complex part 2 part 3\n".to_string()));
        output.insert("stderr".to_string(), Value::String("".to_string()));
        output.insert("exit_code".to_string(), Value::Number(Number::from(0)));
        assert_eq!(result, Some(Ok(Some(Value::Object(output)))));
    }

    #[test]
    fn can_execute_mixed_parameterized_command() {
        let mut executor = BashExecutor {};
        let job_type = make_job_type(Value::String("echo test".to_string()), None);
        let job = make_job(&job_type, Some(Value::Array(vec![Value::String("part 2".to_string()), Value::String("part 3".to_string())])), None);
        let mut context = executor.execute(&job);
        let result = context.result(&job, false);
        let mut output = Map::new();
        output.insert("stdout".to_string(), Value::String("test part 2 part 3\n".to_string()));
        output.insert("stderr".to_string(), Value::String("".to_string()));
        output.insert("exit_code".to_string(), Value::Number(Number::from(0)));
        assert_eq!(result, Some(Ok(Some(Value::Object(output)))));
    }

    #[test]
    fn can_execute_mixed2_parameterized_command() {
        let mut executor = BashExecutor {};
        let job_type = make_job_type(Value::Array(vec![Value::String("echo".to_string()), Value::String("test complex".to_string())]), None);
        // spaces here are ignored when arguments are parsed in bash
        let job = make_job(&job_type, Some(Value::String("part 2    part 3".to_string())), None);
        let mut context = executor.execute(&job);
        let result = context.result(&job, false);
        let mut output = Map::new();
        output.insert("stdout".to_string(), Value::String("test complex part 2 part 3\n".to_string()));
        output.insert("stderr".to_string(), Value::String("".to_string()));
        output.insert("exit_code".to_string(), Value::Number(Number::from(0)));
        assert_eq!(result, Some(Ok(Some(Value::Object(output)))));
    }

    #[test]
    fn can_execute_quoted_command() {
        let mut executor = BashExecutor {};
        let job_type = make_job_type(Value::Array(vec![Value::String("echo".to_string()), Value::String("new\\nline".to_string()), Value::String("\"test\"".to_string())]), None);
        let job = make_job(&job_type, None, None);
        let mut context = executor.execute(&job);
        let result = context.result(&job, false);
        let mut output = Map::new();
        output.insert("stdout".to_string(), Value::String("new\nline \"test\"\n".to_string()));
        output.insert("stderr".to_string(), Value::String("".to_string()));
        output.insert("exit_code".to_string(), Value::Number(Number::from(0)));
        assert_eq!(result, Some(Ok(Some(Value::Object(output)))));
    }

    #[test]
    fn can_execute_command_environment() {
        let mut executor = BashExecutor {};
        let mut environment = Map::new();
        environment.insert("test_env".to_string(), Value::String("test value".to_string()));
        let job_type = make_job_type(Value::String("echo $test_env".to_string()), Some(environment));
        let job = make_job(&job_type, None, None);
        let mut context = executor.execute(&job);
        let result = context.result(&job, false);
        let mut output = Map::new();
        output.insert("stdout".to_string(), Value::String("test value\n".to_string()));
        output.insert("stderr".to_string(), Value::String("".to_string()));
        output.insert("exit_code".to_string(), Value::Number(Number::from(0)));
        assert_eq!(result, Some(Ok(Some(Value::Object(output)))));
    }

    #[test]
    fn can_execute_command_parameterized_environment() {
        let mut executor = BashExecutor {};
        let job_type = make_job_type(Value::String("echo $test_env".to_string()), None);
        let mut environment = Map::new();
        environment.insert("test_env".to_string(), Value::String("test value".to_string()));
        let job = make_job(&job_type, None, Some(environment));
        let mut context = executor.execute(&job);
        let result = context.result(&job, false);
        let mut output = Map::new();
        output.insert("stdout".to_string(), Value::String("test value\n".to_string()));
        output.insert("stderr".to_string(), Value::String("".to_string()));
        output.insert("exit_code".to_string(), Value::Number(Number::from(0)));
        assert_eq!(result, Some(Ok(Some(Value::Object(output)))));
    }

    #[test]
    fn can_execute_command_overridden_environment() {
        let mut executor = BashExecutor {};
        let mut meta_environment = Map::new();
        meta_environment.insert("test_env".to_string(), Value::String("bad test".to_string()));
        let job_type = make_job_type(Value::String("echo $test_env".to_string()), Some(meta_environment));
        let mut environment = Map::new();
        environment.insert("test_env".to_string(), Value::String("test value".to_string()));
        let job = make_job(&job_type, None, Some(environment));
        let mut context = executor.execute(&job);
        let result = context.result(&job, false);
        let mut output = Map::new();
        output.insert("stdout".to_string(), Value::String("test value\n".to_string()));
        output.insert("stderr".to_string(), Value::String("".to_string()));
        output.insert("exit_code".to_string(), Value::Number(Number::from(0)));
        assert_eq!(result, Some(Ok(Some(Value::Object(output)))));
    }

}
