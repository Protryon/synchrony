/*
    router.get("/api/schedules", serialize_wrap(api::schedule::index), "schedule#index");
    router.get("/api/schedules/:uuid", serialize_wrap(api::schedule::get), "schedule#get");
    router.delete("/api/schedules/:uuid", serialize_wrap(api::schedule::get), "schedule#delete");
    router.post("/api/schedules", json_wrap(api::schedule::post), "schedule#post");

*/

use crate::http::middleware::redis::IronRedis;
use iron::prelude::*;
use serde::{Deserialize, Serialize};
use crate::scheduler::ScheduleItem;
use uuid::Uuid;
use std::collections::HashMap;
use serde_json::Value;
use super::{ get_uuid_from_arg, redis_error_translate, option_translate };

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IndexResponse {
    schedules: Vec<ScheduleItem>,
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
    let job_types = redis_error_translate(store.get_job_schedule())?;
    Ok(IndexResponse {
        schedules: job_types,
    })
}

pub fn get(
    req: &mut Request,
    _: &(),
) -> Result<ScheduleItem, IronResult<Response>> {
    let uuid = get_uuid_from_arg(req, "uuid")?;
    let mut store = req.extensions
        .get::<IronRedis>()
        .unwrap()
        .lock()
        .unwrap();
    let schedule_item = option_translate(
        redis_error_translate(store.get_job_schedule_item(uuid))?
    )?;
    Ok(schedule_item)
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PostResponse {
    status: String,
    uuid: Uuid,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PostBody {
    pub interval: u64,
    pub job_type_uuid: Uuid,
    pub job_arguments: HashMap<String, Value>,
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
    option_translate(
        redis_error_translate(store.get_job_type(body.job_type_uuid))?
    )?;
    let schedule_item = ScheduleItem {
        uuid: Uuid::new_v4(),
        interval: body.interval,
        job_type_uuid: body.job_type_uuid,
        job_arguments: body.job_arguments.clone(),
        last_scheduled_at: None,
        last_scheduled_by: None,
    };
    redis_error_translate(store.new_job_schedule_item(&schedule_item))?;
    Ok(PostResponse {
        status: "ok".to_string(),
        uuid: schedule_item.uuid,
    })
}

pub fn delete(
    req: &mut Request,
    _: &(),
) -> Result<PostResponse, IronResult<Response>> {
    let uuid = get_uuid_from_arg(req, "uuid")?;
    let mut store = req.extensions
        .get::<IronRedis>()
        .unwrap()
        .lock()
        .unwrap();
    redis_error_translate(store.delete_job_schedule_item(uuid))?;
    Ok(PostResponse {
        status: "ok".to_string(),
        uuid: uuid,
    })
}
