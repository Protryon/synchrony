use std::process::*;
use crate::exec::executor::*;
use std::env;
use crate::exec::job::Job;
use log::*;
use serde_json::{Value};
use regex::Regex;
use serde_json::map::Map;
use super::bash::BashExecutorContext;
use std::io::{ BufWriter, Write };

pub struct SidekiqExecutor {

}

impl Executor for SidekiqExecutor {
    type Context = BashExecutorContext;

    fn execute(&mut self, job: &Job) -> BashExecutorContext {
        lazy_static! {
            static ref SAFE_WORKER_REGEX: Regex = Regex::new("^[a-zA-Z0-9]+$").unwrap();
        }
        let job_type = job.job_type.as_ref().unwrap();
        let metadata = &job_type.metadata;
        let timeout = job_type.timeout;
        let rails_dir = match metadata.get("rails_dir") {
            Some(Value::String(s)) => {
                s
            },
            _ => {
                error!("No rails_dir found in job type in sidekiq execution for job '{}', job type '{}' / '{}'", job.uuid.hyphenated(), job.job_type.as_ref().unwrap().name, job.job_type_uuid.hyphenated());
                return BashExecutorContext { internal_failure: true, handle: None, timeout: timeout };    
            },
        };
        let sidekiq_worker = match metadata.get("sidekiq_worker") {
            Some(Value::String(s)) => {
                s
            },
            _ => {
                error!("No sidekiq_worker found in job type in sidekiq execution for job '{}', job type '{}' / '{}'", job.uuid.hyphenated(), job.job_type.as_ref().unwrap().name, job.job_type_uuid.hyphenated());
                return BashExecutorContext { internal_failure: true, handle: None, timeout: timeout };    
            },
        };
        if !SAFE_WORKER_REGEX.is_match(sidekiq_worker) {
            error!("Invalid sidekiq_worker for job '{}', job type '{}' / '{}': '{}'", job.uuid.hyphenated(), job.job_type.as_ref().unwrap().name, job.job_type_uuid.hyphenated(), sidekiq_worker);
            return BashExecutorContext { internal_failure: true, handle: None, timeout: timeout };    
        }
        let ruby_executable = match metadata.get("ruby_executable") {
            Some(Value::String(s)) => {
                s
            },
            None => {
                "ruby"
            },
            _ => {
                warn!("No valid ruby_executable described for job '{}', job type '{}' / '{}', using 'ruby'", job.uuid.hyphenated(), job.job_type.as_ref().unwrap().name, job.job_type_uuid.hyphenated());
                "ruby"
            },
        };

        let sidekiq_arguments = match job.arguments.get("sidekiq_arguments") {
            Some(value) => {
                Some(serde_json::to_string(&value).unwrap())
            },
            _ => {
                None
            },
        };

        let meta_environment = metadata.get("environment");
        let environment = job.arguments.get("environment");
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
        let mut builder = Command::new(env::var("SHELL").unwrap_or("/bin/bash".to_string()));
        builder
            .stderr(Stdio::piped())
            .stdout(Stdio::piped())
            .arg("-c")
            .stdin(Stdio::piped())
            .current_dir(rails_dir);
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
        builder.arg(ruby_executable);
        let handle = builder.spawn();
        if handle.is_err() {
            error!("Failed to spawn bash in bash execution for job '{}', job type '{}' / '{}': {:?}", job.uuid.hyphenated(), job.job_type.as_ref().unwrap().name, job.job_type_uuid.hyphenated(), handle.unwrap_err());
            return BashExecutorContext { internal_failure: true, handle: None, timeout: timeout };
        }
        let mut handle = handle.unwrap();
        {
            let stdin = handle
                .stdin
                .as_mut()
                .expect("failed to open stdin for sidekiq executor");
            let mut stdin_writer = BufWriter::new(stdin);
            stdin_writer
                .write_all(format!("
                    require './config/environment'
                    {}.new.perform({})
                ", sidekiq_worker, sidekiq_arguments.map(|args| format!("JSON.parse('{}')", args.replace('\\', "\\\\").replace("'", "\\'"))).unwrap_or("nil".to_string())).as_bytes())
                .expect("failed to write to sidekiq executor stdin");
            stdin_writer
                .flush()
                .expect("failed to write to sidekiq executor stdin");
        }
        drop(handle.stdin.take());
        return BashExecutorContext { internal_failure: false, handle: Some(handle), timeout: timeout }
    }
}


#[cfg(test)]
mod tests {
    use super::*;
    use uuid::Uuid;
    use crate::exec::job_type::JobType;
    use std::collections::HashMap;
    use serde_json::Number;

    fn make_job_type(environment: Option<Map<String, Value>>) -> JobType {
        let mut job_type = JobType {
            executor: "sidekiq".to_string(),
            name: "test".to_string(),
            node_type: "default".to_string(),
            timeout: None,
            unique: false,
            uuid: Uuid::new_v4(),
            metadata: HashMap::new(),
        };
        job_type.metadata.insert("rails_dir".to_string(), Value::String("./rails_test".to_string()));
        job_type.metadata.insert("sidekiq_worker".to_string(), Value::String("TestWorker".to_string()));
        
        if environment.is_some() {
            job_type.metadata.insert("environment".to_string(), Value::Object(environment.unwrap()));
        }
        return job_type;
    }

    fn make_job(job_type: &JobType, arguments: Option<Value>, environment: Option<Map<String, Value>>) -> Job {
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
        if arguments.is_some() {
            job.arguments.insert("sidekiq_arguments".to_string(), arguments.unwrap());
        }
        if environment.is_some() {
            job.arguments.insert("environment".to_string(), Value::Object(environment.unwrap()));
        }
        return job;
    }

    fn init_rails() {
        Command::new("./bin/setup_rails_test.sh").output().expect("failed to initialize rails environment for testing");
    }

    fn rails_test(job_type: &JobType, value: Option<Value>, raw_value: &'static str) {
        let job = make_job(job_type, value, None);
        let mut context = (SidekiqExecutor {}).execute(&job);
        let result = context.result(&job, false);
        let mut output = Map::new();
        output.insert("stdout".to_string(), Value::String(format!("test successful {}\n", raw_value)));
        output.insert("stderr".to_string(), Value::String("".to_string()));
        output.insert("exit_code".to_string(), Value::Number(Number::from(0)));
        assert_eq!(result, Some(Ok(Some(Value::Object(output)))));
    }

    // if this test is failing, it probably has to due with a lack of rails installation
    #[test]
    fn can_execute_sidekiq_job() {
        init_rails();
        let job_type = make_job_type(None);
        rails_test(&job_type, None, "null");
        rails_test(&job_type, Some(Value::Bool(true)), "true");
        let mut map = Map::new();
        map.insert("test_key".to_string(), Value::String("test_value".to_string()));
        rails_test(&job_type, Some(Value::Object(map)), "{\"test_key\":\"test_value\"}");
    }

}