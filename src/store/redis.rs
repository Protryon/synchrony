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
        let new_node = Node { uuid: Uuid::new_v4(), last_ping: epoch(), node_type_uuid: None, node_type: None };
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

    fn new_node_type(&mut self, node_type: &NodeType) -> Result<(), String> {
        let redis_result: Result<(), ::redis::RedisError> = self.connection.hset("node_types", node_type.name.clone(), serde_json::to_string(&node_type).unwrap());
        if redis_result.is_err() {
            return Err(format!("{:?}", redis_result.err().unwrap()));
        }
        return Ok(());
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
    
    fn get_job_type(&mut self, uuid: Uuid) -> Result<Option<JobType>, String> {
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

    fn new_job_type(&mut self, job_type: &JobType) -> Result<(), String> {
        let redis_result: Result<(), ::redis::RedisError> = self.connection.hset("job_types", job_type.uuid.hyphenated().to_string(), serde_json::to_string(&job_type).unwrap());
        if redis_result.is_err() {
            return Err(format!("{:?}", redis_result.err().unwrap()));
        }
        return Ok(());
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

    fn new_job_schedule_item(&mut self, schedule_item: &ScheduleItem) -> Result<(), String> {
        let redis_result: Result<(), ::redis::RedisError> = self.connection.hset("schedule_items", schedule_item.uuid.hyphenated().to_string(), serde_json::to_string(&schedule_item).unwrap());
        if redis_result.is_err() {
            return Err(format!("{:?}", redis_result.err().unwrap()));
        }
        return Ok(());
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
        let redis_result: Result<u32, ::redis::RedisError> = self.connection.rpush(format!("jobs_waiting_{}", node_type_uuid), serde_json::to_string(&job).unwrap());
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

    fn get_all_jobs_in_progress(&mut self) -> Result<Vec<Job>, String> {
        let node_type_uuid = self.node.node_type_uuid.unwrap().hyphenated().to_string();
        return self.get_all_jobs_in(format!("jobs_in_progress_{}", node_type_uuid));
    }

    fn get_all_jobs_finished(&mut self) -> Result<Vec<Job>, String> {
        let node_type_uuid = self.node.node_type_uuid.unwrap().hyphenated().to_string();
        return self.get_all_jobs_in(format!("jobs_finished_{}", node_type_uuid));
    }

    fn get_finished_job(&mut self, uuid: Uuid) -> Result<Job, String> {
        let node_type_uuid = self.node.node_type_uuid.unwrap().hyphenated().to_string();
        let redis_result: Result<String, ::redis::RedisError> = self.connection.hget(format!("jobs_finished_{}", node_type_uuid), uuid.hyphenated().to_string());
        if redis_result.is_err() {
            return Err(format!("{:?}", redis_result.err().unwrap()));
        }
        let raw_job: Result<Job, serde_json::Error> = serde_json::from_str(&*redis_result.unwrap());
        if raw_job.is_err() {
            return Err(format!("{:?}", raw_job.err().unwrap()));
        }
        let mut job = raw_job.unwrap();
        job.job_type = Some(self.get_cached_job_type(job.job_type_uuid)?);
        return Ok(job);
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
    
    fn clean(&mut self) {
        let _: Result<(), ::redis::RedisError> = ::redis::cmd("FLUSHDB").query(&mut self.connection);
    }
}

impl RedisStore {

    fn get_all_jobs_in(&mut self, key: String) -> Result<Vec<Job>, String> {
        let node_type_uuid = self.node.node_type_uuid.unwrap().hyphenated().to_string();
        let redis_result: Result<Vec<String>, ::redis::RedisError> = self.connection.hgetall(key);
        if redis_result.is_err() {
            return Err(format!("{:?}", redis_result.err().unwrap()));
        }
        let raw_redis = redis_result.unwrap();
        let mut output: Vec<Job> = vec![];
        let mut current_uuid: &String = &"".to_string();
        for i in 0..raw_redis.len() {
            if i % 2 == 0 {
                current_uuid = &raw_redis[i];
            } else {
                let raw_job: Result<Job, serde_json::Error> = serde_json::from_str(&*raw_redis[i]);
                if raw_job.is_err() {
                    return Err(format!("{:?}", raw_job.err().unwrap()));
                }
                let mut job = raw_job.unwrap();
                if job.uuid.hyphenated().to_string() != *current_uuid {
                    return Err(format!("redis consistency error: hash key '{}' not equal data given uuid '{}'", job.uuid.hyphenated(), *current_uuid));
                }
                job.job_type = Some(self.get_cached_job_type(job.job_type_uuid)?);
                output.push(job);
            }
        }
        return Ok(output);
    }

    fn get_cached_job_type(&mut self, uuid: Uuid) -> Result<JobType, String> {
        let cached = self.job_types.get(&uuid);
        if cached.is_some() {
            return Ok(cached.unwrap().clone());
        }
        let retrieved = self.get_job_type(uuid)?;
        if retrieved.is_none() {
            return Err(format!("invalid job type: '{}'", uuid.hyphenated().to_string()));
        }
        self.job_types.insert(uuid, retrieved.as_ref().unwrap().clone());
        return Ok(retrieved.unwrap());
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::store::tests::*;

    #[test]
    fn can_connect() -> Result<(), String> {
        let _store = RedisStore::connect()?;
        // we successfully connected (this test is mostly to ensure redis is available in the testing environment)
        Ok(())
    }

    #[test]
    fn can_get_node_types() -> Result<(), String> {
        let mut store: StoreRef = Box::new(RedisStore::connect()?);
        store.clean();
        let node_types = store.get_node_types()?;
        assert_eq!(node_types, vec![]);
        let test_node_type = make_node_type(&mut store)?;
        let node_types_new = store.get_node_types()?;
        assert_eq!(node_types_new, vec![test_node_type]);
        Ok(())
    }

    #[test]
    fn can_set_node_type() -> Result<(), String> {
        let mut store = RedisStore::connect()?;
        let mut boxed_store: StoreRef = store.replicate()?;
        boxed_store.clean();
        let test_node_type = make_node_type(&mut boxed_store)?;
        boxed_store.set_node_type(test_node_type.uuid)?;
        let redis_result: Result<String, ::redis::RedisError> = store.connection.hget("nodes", boxed_store.get_node().uuid.hyphenated().to_string());
        assert_eq!(redis_result.unwrap(), serde_json::to_string(boxed_store.get_node()).unwrap());
        Ok(())
    }

    #[test]
    fn can_get_job_types() -> Result<(), String> {
        let mut store: StoreRef = Box::new(RedisStore::connect()?);
        store.clean();
        let test_node_type = make_node_type(&mut store)?;
        store.set_node_type(test_node_type.uuid)?;
        let job_types = store.get_job_types()?;
        assert_eq!(job_types, vec![]);
        let test_job_type = make_job_type(&mut store)?;
        let job_types_new = store.get_job_types()?;
        assert_eq!(job_types_new, vec![test_job_type]);
        Ok(())
    }

    #[test]
    fn can_get_job_schedule() -> Result<(), String> {
        let mut store: StoreRef = Box::new(RedisStore::connect()?);
        store.clean();
        let test_node_type = make_node_type(&mut store)?;
        store.set_node_type(test_node_type.uuid)?;
        let schedule = store.get_job_schedule()?;
        assert_eq!(schedule, vec![]);
        let test_job_type = make_job_type(&mut store)?;
        let test_schedule_item = make_schedule_item(&mut store, test_job_type.uuid)?;
        let schedule_new = store.get_job_schedule()?;
        assert_eq!(schedule_new, vec![test_schedule_item]);
        Ok(())
    }

    #[test]
    fn can_claim_job_schedule() -> Result<(), String> {
        let mut store: StoreRef = Box::new(RedisStore::connect()?);
        store.clean();
        let test_node_type = make_node_type(&mut store)?;
        store.set_node_type(test_node_type.uuid)?;
        let test_job_type = make_job_type(&mut store)?;
        let mut test_schedule_item = make_schedule_item(&mut store, test_job_type.uuid)?;
        let claimed = store.claim_job_scheduled(&test_schedule_item)?;
        assert_ne!(claimed, Some(test_schedule_item.clone()));
        assert_ne!(claimed, None);
        test_schedule_item.last_scheduled_at = claimed.as_ref().unwrap().last_scheduled_at;
        test_schedule_item.last_scheduled_by = claimed.as_ref().unwrap().last_scheduled_by;
        assert_eq!(claimed, Some(test_schedule_item));
        Ok(())
    }

    #[test]
    fn can_enqueue_dequeue_jobs() -> Result<(), String> {
        let mut store: StoreRef = Box::new(RedisStore::connect()?);
        store.clean();
        let test_node_type = make_node_type(&mut store)?;
        store.set_node_type(test_node_type.uuid)?;
        let test_job_type = make_job_type(&mut store)?;
        let mut test_job = make_job(&mut store, &test_job_type)?;
        assert_eq!(test_job.enqueued_at, None);
        assert_eq!(test_job.started_at, None);
        assert_eq!(test_job.executing_node, None);
        let dequeued_job = store.dequeue_job()?;
        test_job.enqueued_at = dequeued_job.enqueued_at;
        test_job.started_at = dequeued_job.started_at;
        test_job.executing_node = dequeued_job.executing_node;
        let all_jobs_waiting = store.get_all_jobs_in_progress()?;
        assert_eq!(all_jobs_waiting, vec![test_job.clone()]);
        assert_eq!(dequeued_job, test_job);
        Ok(())
    }

    #[test]
    fn can_enqueue_dequeue_finish_jobs() -> Result<(), String> {
        let mut store: StoreRef = Box::new(RedisStore::connect()?);
        store.clean();
        let test_node_type = make_node_type(&mut store)?;
        store.set_node_type(test_node_type.uuid)?;
        let test_job_type = make_job_type(&mut store)?;
        make_job(&mut store, &test_job_type)?;
        let dequeued_job = store.dequeue_job()?;
        store.finish_job(dequeued_job.clone(), Some(Value::Bool(true)), Some(Value::Bool(false)))?;
        let all_jobs_finished = store.get_all_jobs_finished()?;
        let mut theoretical_finished_job = dequeued_job.clone();
        assert_eq!(all_jobs_finished.len(), 1);
        theoretical_finished_job.ended_at = all_jobs_finished[0].ended_at;
        theoretical_finished_job.results = Some(Value::Bool(true));
        theoretical_finished_job.errors = Some(Value::Bool(false));
        assert_eq!(all_jobs_finished, vec![theoretical_finished_job]);
        Ok(())
    }

    #[test]
    fn can_ping() -> Result<(), String> {
        let mut store = RedisStore::connect()?;
        let mut boxed_store: StoreRef = store.replicate()?;
        boxed_store.clean();
        let test_node_type = make_node_type(&mut boxed_store)?;
        boxed_store.set_node_type(test_node_type.uuid)?;
        boxed_store.ping()?;
        let redis_result: Result<String, ::redis::RedisError> = store.connection.hget("nodes".to_string(), boxed_store.get_node().uuid.hyphenated().to_string());
        if redis_result.is_err() {
            return Err(format!("{:?}", redis_result.err().unwrap()));
        }
        let raw_node: Result<Node, serde_json::Error> = serde_json::from_str(&*redis_result.unwrap());
        if raw_node.is_err() {
            return Err(format!("{:?}", raw_node.err().unwrap()));
        }
        assert_eq!(raw_node.unwrap(), *boxed_store.get_node());
        Ok(())
    }

}