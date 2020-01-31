use crate::store::*;
use crate::util::config;
use ::redis::{ Client, Connection, Commands };
use crate::exec::node::Node;
use uuid::Uuid;
use serde_json::Value;
use crate::util::time::epoch;
use std::collections::HashMap;

pub struct RedisStore {
    client: Client,
    connection: Connection,
    node: Node,
    ping_interval: u32,
    job_types: HashMap<Uuid, JobType>,
}

fn redis_hcheck_set(connection: &mut Connection, key: String, hkey: String, old_value: Option<String>, new_value: Option<String>) -> Result<bool, String> {
    let mut command = ::redis::cmd("EVAL");
    let mut builder = command.arg("
        local c = tostring(redis.call('hget', KEYS[1], KEYS[2]));
        if c == ARGV[1] then
            redis.call('hset', KEYS[1], KEYS[2], ARGV[2]);
            return 'true';
        end
        return 'false';
    ").arg(2).arg(key).arg(hkey);
    builder = match old_value {
        None => builder.arg(false),
        Some(s) => builder.arg(s),
    };
    builder = match new_value {
        None => builder.arg(false),
        Some(s) => builder.arg(s),
    };
    let redis_result: Result<Option<String>, ::redis::RedisError> = builder.query(connection);
    if redis_result.is_err() {
        return Err(format!("{:?}", redis_result.err().unwrap()));
    }
    return Ok(redis_result.unwrap().unwrap_or("false".to_string()) == "true");
}

impl Store for RedisStore {
    fn connect() -> Result<RedisStore, String> {
        let client = Client::open(&*format!("redis://{}:{}/{}", &*config::REDIS_HOST, &*config::REDIS_PORT, &*config::REDIS_DATABASE));
        if client.is_err() {
            return Err(format!("{:?}", client.unwrap_err()));
        }
        let mut connection = client.as_ref().unwrap().get_connection();
        if connection.is_err() {
            return Err(format!("{:?}", connection.err().unwrap()));
        }
        let new_node = Node { uuid: Uuid::new_v4(), last_ping: epoch(), node_type_uuid: None };
        let redis_result: Result<(), ::redis::RedisError> = connection.as_mut().unwrap().hset("nodes", new_node.uuid.hyphenated().to_string(), serde_json::to_string(&new_node).unwrap());
        if redis_result.is_err() {
            return Err(format!("{:?}", redis_result.err().unwrap()));
        }
        return Ok(RedisStore { client: client.unwrap(), connection: connection.unwrap(), node: new_node, ping_interval: 5000, job_types: HashMap::new() });
    }

    fn get_node_types(&mut self) -> Result<Vec<NodeType>, String> {
        let redis_result: Result<Vec<String>, ::redis::RedisError> = self.connection.hgetall("node_types");
        if redis_result.is_err() {
            return Err(format!("{:?}", redis_result.err().unwrap()));
        }
        let raw_redis = redis_result.unwrap();
        let mut output: Vec<NodeType> = vec![];
        let mut current_name: &String = &"".to_string();
        for i in 0..raw_redis.len() {
            if i % 2 == 0 {
                current_name = &raw_redis[i];
            } else {
                let raw_node_type: Result<NodeType, serde_json::Error> = serde_json::from_str(&*raw_redis[i]);
                if raw_node_type.is_err() {
                    return Err(format!("{:?}", raw_node_type.err().unwrap()));
                }
                let node_type = raw_node_type.unwrap();
                if node_type.name != *current_name {
                    return Err(format!("redis consistency error: hash key '{}' not equal data given name '{}'", node_type.name, *current_name));
                }
                output.push(node_type);
            }
        }
        return Ok(output);
    }

    fn set_node_type(&mut self, node_type_uuid: Uuid) -> Result<(), String> {
        self.node.node_type_uuid = Some(node_type_uuid);
        let redis_result: Result<(), ::redis::RedisError> = self.connection.hset("nodes", self.node.uuid.hyphenated().to_string(), serde_json::to_string(&self.node).unwrap());
        if redis_result.is_err() {
            return Err(format!("{:?}", redis_result.err().unwrap()));
        }
        return Ok(());
    }

    fn get_job_types(&mut self) -> Result<Vec<JobType>, String> {
        let redis_result: Result<Vec<String>, ::redis::RedisError> = self.connection.hgetall("job_types");
        if redis_result.is_err() {
            return Err(format!("{:?}", redis_result.err().unwrap()));
        }
        let raw_redis = redis_result.unwrap();
        let mut output: Vec<JobType> = vec![];
        let mut current_name: &String = &"".to_string();
        for i in 0..raw_redis.len() {
            if i % 2 == 0 {
                current_name = &raw_redis[i];
            } else {
                let raw_job_type: Result<JobType, serde_json::Error> = serde_json::from_str(&*raw_redis[i]);
                if raw_job_type.is_err() {
                    return Err(format!("{:?}", raw_job_type.err().unwrap()));
                }
                let job_type = raw_job_type.unwrap();
                let hyphenated_uuid = job_type.uuid.hyphenated().to_string();
                if hyphenated_uuid != *current_name {
                    return Err(format!("redis consistency error: hash key '{}' not equal data given uuid '{}'", hyphenated_uuid, *current_name));
                }
                output.push(job_type);
            }
        }
        return Ok(output);
    }
    
    fn get_new_job_type(&mut self, uuid: Uuid) -> Result<Option<JobType>, String> {
        let redis_result: Result<Option<String>, ::redis::RedisError> = self.connection.hget("job_types", uuid.hyphenated().to_string());
        if redis_result.is_err() {
            return Err(format!("{:?}", redis_result.err().unwrap()));
        }
        if redis_result.as_ref().unwrap().is_none() {
            return Ok(None);
        }
        let raw_job_type: Result<JobType, serde_json::Error> = serde_json::from_str(&*redis_result.unwrap().unwrap());
        if raw_job_type.is_err() {
            return Err(format!("{:?}", raw_job_type.err().unwrap()));
        }
        return Ok(Some(raw_job_type.unwrap()));
    }

    fn get_job_schedule(&mut self) -> Result<Vec<ScheduleItem>, String> {
        let redis_result: Result<Vec<String>, ::redis::RedisError> = self.connection.hgetall("schedule_items");
        if redis_result.is_err() {
            return Err(format!("{:?}", redis_result.err().unwrap()));
        }
        let raw_redis = redis_result.unwrap();
        let mut output: Vec<ScheduleItem> = vec![];
        let mut current_name: &String = &"".to_string();
        for i in 0..raw_redis.len() {
            if i % 2 == 0 {
                current_name = &raw_redis[i];
            } else {
                let raw_schedule_item: Result<ScheduleItem, serde_json::Error> = serde_json::from_str(&*raw_redis[i]);
                if raw_schedule_item.is_err() {
                    return Err(format!("{:?}", raw_schedule_item.err().unwrap()));
                }
                let schedule_item = raw_schedule_item.unwrap();
                let hyphenated_uuid = schedule_item.uuid.hyphenated().to_string();
                if hyphenated_uuid != *current_name {
                    return Err(format!("redis consistency error: hash key '{}' not equal data given uuid '{}'", hyphenated_uuid, *current_name));
                }
                output.push(schedule_item);
            }
        }
        return Ok(output);
    }
    
    fn claim_job_scheduled(&mut self, schedule_item: &ScheduleItem) -> Result<Option<ScheduleItem>, String> {
        let mut new_schedule_item = schedule_item.clone();
        new_schedule_item.last_scheduled_by = Some(self.node.uuid);
        new_schedule_item.last_scheduled_at = Some(epoch());
        let updated = redis_hcheck_set(&mut self.connection, "schedule_items".to_string(), schedule_item.uuid.hyphenated().to_string(), Some(serde_json::to_string(schedule_item).unwrap()), Some(serde_json::to_string(&new_schedule_item).unwrap()))?;
        if updated {
            return Ok(Some(new_schedule_item)); // we claimed it
        } else {
            return Ok(None); // someone else modified/claimed it
        }
    }
    
    fn enqueue_job(&mut self, mut job: Job) -> Result<(), String> {
        job.enqueued_at = Some(epoch());
        let node_type_uuid = self.node.node_type_uuid.unwrap().hyphenated().to_string();
        let redis_result: Result<String, ::redis::RedisError> = self.connection.rpush(format!("jobs_waiting_{}", node_type_uuid), serde_json::to_string(&job).unwrap());
        if redis_result.is_err() {
            return Err(format!("{:?}", redis_result.err().unwrap()));
        }
        return Ok(());
    }
    
    fn dequeue_job(&mut self) -> Result<Job, String> {
        let node_type_uuid = self.node.node_type_uuid.unwrap().hyphenated().to_string();
        let redis_result: Result<Vec<String>, ::redis::RedisError> = self.connection.blpop(format!("jobs_waiting_{}", node_type_uuid), 0);
        if redis_result.is_err() {
            return Err(format!("{:?}", redis_result.err().unwrap()));
        }
        let raw_job: Result<Job, serde_json::Error> = serde_json::from_str(&*(redis_result.unwrap())[1]);
        if raw_job.is_err() {
            return Err(format!("{:?}", raw_job.err().unwrap()));
        }
        let mut job = raw_job.unwrap();
        job.started_at = Some(epoch());
        job.executing_node = Some(self.node.node_type_uuid.unwrap());
        let redis_result: Result<(), ::redis::RedisError> = self.connection.hset(format!("jobs_in_progress_{}", node_type_uuid), job.uuid.hyphenated().to_string(), serde_json::to_string(&job).unwrap());
        if redis_result.is_err() {
            return Err(format!("{:?}", redis_result.err().unwrap()));
        }
        job.job_type = Some(self.get_cached_job_type(job.job_type_uuid)?);
        return Ok(job);
    }
    
    fn finish_job(&mut self, mut job: Job, results: Option<Value>, errors: Option<Value>) -> Result<(), String> {
        let node_type_uuid = self.node.node_type_uuid.unwrap().hyphenated().to_string();
        let redis_result: Result<(), ::redis::RedisError> = self.connection.hdel(format!("jobs_in_progress_{}", node_type_uuid), job.uuid.hyphenated().to_string());
        if redis_result.is_err() {
            return Err(format!("{:?}", redis_result.err().unwrap()));
        }
        job.ended_at = Some(epoch());
        job.results = results;
        job.errors = errors;
        let redis_result: Result<(), ::redis::RedisError> = self.connection.hset(format!("jobs_finished_{}", node_type_uuid), job.uuid.hyphenated().to_string(), serde_json::to_string(&job).unwrap());
        if redis_result.is_err() {
            return Err(format!("{:?}", redis_result.err().unwrap()));
        }
        return Ok(());
    }

    fn ping(&mut self) -> Result<(), String> {
        self.node.last_ping = epoch();
        let redis_result: Result<(), ::redis::RedisError> = self.connection.hset("nodes", self.node.uuid.hyphenated().to_string(), serde_json::to_string(&self.node).unwrap());
        if redis_result.is_err() {
            return Err(format!("{:?}", redis_result.err().unwrap()));
        }
        return Ok(());
    }

    fn get_ping_interval_ms(&self) -> u32 {
        return self.ping_interval;
    }

    fn get_node(&mut self) -> &mut Node {
        return &mut self.node;
    }

    fn replicate(&self) -> Result<StoreRef, String> {
        let new_client = self.client.clone();
        let connection = new_client.get_connection();
        if connection.is_err() {
            return Err(format!("{:?}", connection.err().unwrap()));
        }
        return Ok(Box::new(RedisStore {
            client: new_client,
            connection: connection.unwrap(),
            ping_interval: self.ping_interval,
            node: self.node.clone(),
            job_types: self.job_types.clone(),
        }));
    }
    
}

impl RedisStore {

    fn get_cached_job_type(&mut self, uuid: Uuid) -> Result<JobType, String> {
        let cached = self.job_types.get(&uuid);
        if cached.is_some() {
            return Ok(cached.unwrap().clone());
        }
        let retrieved = self.get_new_job_type(uuid)?;
        if retrieved.is_none() {
            return Err(format!("invalid job type: '{}'", uuid.hyphenated().to_string()));
        }
        self.job_types.insert(uuid, retrieved.as_ref().unwrap().clone());
        return Ok(retrieved.unwrap());
    }
}