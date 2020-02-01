
/*
    router.get("/api/jobs/:node_type_uuid/:uuid", serialize_wrap(api::jobs::get), "jobs#get");
    router.post("/api/jobs/:node_type_uuid", json_wrap(api::jobs::post), "jobs#post");
*/

use crate::http::middleware::redis::IronRedis;
use iron::prelude::*;
use serde::{Deserialize, Serialize};
use crate::exec::job::Job;
use uuid::Uuid;
use std::collections::HashMap;
use serde_json::Value;
use super::{ get_uuid_from_arg, redis_error_translate, option_translate };

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IndexResponse {
    jobs: Vec<Job>,
}

pub fn index_queued(
    req: &mut Request,
    _: &(),
) -> Result<IndexResponse, IronResult<Response>> {
    let mut store = req.extensions
        .get::<IronRedis>()
        .unwrap()
        .lock()
        .unwrap();
    let current_node_type_uuid = store.get_node().uuid;
    let node_type_uuid = get_uuid_from_arg(req, "node_type_uuid")?;
    option_translate(redis_error_translate(store.set_node_type_soft(node_type_uuid))?)?;
    let jobs = redis_error_translate(store.get_all_jobs_waiting());
    option_translate(redis_error_translate(store.set_node_type_soft(current_node_type_uuid))?)?;
    if jobs.is_err() {
        return Err(jobs.err().unwrap());
    }
    Ok(IndexResponse {
        jobs: jobs.unwrap(),
    })
}

pub fn index_in_progress(
    req: &mut Request,
    _: &(),
) -> Result<IndexResponse, IronResult<Response>> {
    let mut store = req.extensions
        .get::<IronRedis>()
        .unwrap()
        .lock()
        .unwrap();
    let current_node_type_uuid = store.get_node().uuid;
    let node_type_uuid = get_uuid_from_arg(req, "node_type_uuid")?;
    option_translate(redis_error_translate(store.set_node_type_soft(node_type_uuid))?)?;
    let jobs = redis_error_translate(store.get_all_jobs_in_progress());
    option_translate(redis_error_translate(store.set_node_type_soft(current_node_type_uuid))?)?;
    if jobs.is_err() {
        return Err(jobs.err().unwrap());
    }
    Ok(IndexResponse {
        jobs: jobs.unwrap(),
    })
}

pub fn index_finished(
    req: &mut Request,
    _: &(),
) -> Result<IndexResponse, IronResult<Response>> {
    let mut store = req.extensions
        .get::<IronRedis>()
        .unwrap()
        .lock()
        .unwrap();
    let current_node_type_uuid = store.get_node().uuid;
    let node_type_uuid = get_uuid_from_arg(req, "node_type_uuid")?;
    option_translate(redis_error_translate(store.set_node_type_soft(node_type_uuid))?)?;
    let jobs = redis_error_translate(store.get_all_jobs_finished());
    option_translate(redis_error_translate(store.set_node_type_soft(current_node_type_uuid))?)?;
    if jobs.is_err() {
        return Err(jobs.err().unwrap());
    }
    let jobs_unwrapped = jobs.unwrap().iter().map(|job| {
        Job {
            uuid: job.uuid,
            job_type_uuid: job.job_type_uuid,
            job_type: None,
            arguments: job.arguments.clone(),
            executing_node: job.executing_node,
            enqueued_at: job.enqueued_at,
            started_at: job.started_at,
            ended_at: job.ended_at,
            results: Some(Value::Bool(job.results.is_some())),
            errors: Some(Value::Bool(job.errors.is_some())),
        }
    }).collect();
    Ok(IndexResponse {
        jobs: jobs_unwrapped,
    })
}

pub fn get(
    req: &mut Request,
    _: &(),
) -> Result<Job, IronResult<Response>> {
    let node_type_uuid = get_uuid_from_arg(req, "node_type_uuid")?;
    let job_uuid = get_uuid_from_arg(req, "uuid")?;
    let mut store = req.extensions
        .get::<IronRedis>()
        .unwrap()
        .lock()
        .unwrap();
    let current_node_type_uuid = store.get_node().uuid;
    option_translate(redis_error_translate(store.set_node_type_soft(node_type_uuid))?)?;
    let job = redis_error_translate(store.get_finished_job(job_uuid));
    option_translate(redis_error_translate(store.set_node_type_soft(current_node_type_uuid))?)?;
    if job.is_err() {
        return Err(job.err().unwrap());
    }
    Ok(option_translate(job.unwrap())?)
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PostResponse {
    status: String,
    uuid: Uuid,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PostBody {
    pub job_type_uuid: Uuid,
    pub arguments: HashMap<String, Value>,
}

pub fn post(
    req: &mut Request,
    body: &PostBody,
) -> Result<PostResponse, IronResult<Response>> {
    let mut store = req.extensions
        .get::<IronRedis>()
        .unwrap()
        .lock()
        .unwrap();
    let job_type = option_translate(redis_error_translate(store.get_job_type(body.job_type_uuid))?)?;
    let job_uuid = Uuid::new_v4();
    let job = Job {
        uuid: job_uuid,
        job_type_uuid: body.job_type_uuid,
        job_type: Some(job_type),
        arguments: body.arguments.clone(),
        executing_node: None,
        enqueued_at: None,
        started_at: None,
        ended_at: None,
        results: None,
        errors: None,
    };
    redis_error_translate(store.enqueue_job(job))?;
    Ok(PostResponse {
        status: "ok".to_string(),
        uuid: job_uuid,
    })
}
