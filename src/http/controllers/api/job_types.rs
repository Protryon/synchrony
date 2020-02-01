use crate::http::middleware::redis::IronRedis;
use iron::prelude::*;
use serde::{Deserialize, Serialize};
use crate::exec::job_type::JobType;
use uuid::Uuid;
use std::collections::HashMap;
use serde_json::Value;
use super::{ get_uuid_from_arg, redis_error_translate, option_translate };

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IndexResponse {
    job_types: Vec<JobType>,
}

pub fn index(
    req: &mut Request,
    _: &(),
) -> Result<IndexResponse, IronResult<Response>> {
    let mut store = req.extensions
        .get::<IronRedis>()
        .unwrap()
        .lock()
        .unwrap();
    let job_types = redis_error_translate(store.get_job_types())?;
    Ok(IndexResponse {
        job_types: job_types,
    })
}

pub fn get(
    req: &mut Request,
    _: &(),
) -> Result<JobType, IronResult<Response>> {
    let uuid = get_uuid_from_arg(req, "uuid")?;
    let mut store = req.extensions
        .get::<IronRedis>()
        .unwrap()
        .lock()
        .unwrap();
    let job_type = option_translate(
        redis_error_translate(store.get_job_type(uuid))?
    )?;
    Ok(job_type)
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PostResponse {
    status: String,
    uuid: Uuid,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PostBody {
    pub name: String,
    pub executor: String,
    pub metadata: HashMap<String, Value>,
    pub unique: bool,
    pub node_type: String,
    pub timeout: Option<u64>,
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
    let job_type = JobType {
        uuid: Uuid::new_v4(),
        name: body.name.clone(),
        executor: body.executor.clone(),
        metadata: body.metadata.clone(),
        unique: body.unique,
        node_type: body.node_type.clone(),
        timeout: body.timeout,
    };
    redis_error_translate(store.new_job_type(&job_type))?;
    Ok(PostResponse {
        status: "ok".to_string(),
        uuid: job_type.uuid,
    })
}
