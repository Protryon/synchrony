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

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
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

#[cfg(test)]
mod tests {
    use super::*;
    use iron_test::request::{ post, get, delete };
    use iron::{ Headers, headers::ContentType };
    use crate::http::controllers::tests::*;
    use crate::config;
    use iron::status;
    use crate::http::tests::initialize_tests;
    use crate::store::{ self, StoreRef, tests::* };

    #[test]
    fn test_schedule_item_index() -> Result<(), String> {
        let mut store: StoreRef = store::init_store_untyped();
        store.clean();
        let test_node_type = make_node_type(&mut store)?;
        store.set_node_type(test_node_type.uuid)?;
        let test_job_type = make_job_type(&mut store)?;
        let test_schedule = make_schedule_item(&mut store, test_job_type.uuid, None, None)?;

        let response = iron_error_translate(get(&*format!("http://{}/api/schedules", &*config::HTTP_BIND_ADDRESS), Headers::new(), &initialize_tests(store)))?;
        assert_eq!(response.status, Some(status::Ok));
        let body: IndexResponse = parse_body(response.body)?;
        assert_eq!(body, IndexResponse {
            schedules: vec![test_schedule],
        });
        Ok(())
    }

    #[test]
    fn test_schedule_item_get() -> Result<(), String> {
        let mut store: StoreRef = store::init_store_untyped();
        store.clean();
        let test_node_type = make_node_type(&mut store)?;
        store.set_node_type(test_node_type.uuid)?;
        let test_job_type = make_job_type(&mut store)?;
        let test_schedule = make_schedule_item(&mut store, test_job_type.uuid, None, None)?;

        let response = iron_error_translate(get(&*format!("http://{}/api/schedules/{}", &*config::HTTP_BIND_ADDRESS, test_schedule.uuid.hyphenated()), Headers::new(), &initialize_tests(store)))?;
        assert_eq!(response.status, Some(status::Ok));
        let body: ScheduleItem = parse_body(response.body)?;
        assert_eq!(body, test_schedule);
        Ok(())
    }

    #[test]
    fn test_schedule_item_post() -> Result<(), String> {
        let mut store: StoreRef = store::init_store_untyped();
        store.clean();
        let test_node_type = make_node_type(&mut store)?;
        store.set_node_type(test_node_type.uuid)?;
        let test_job_type = make_job_type(&mut store)?;

        let test_schedule_item = PostBody {
            interval: 1000,
            job_type_uuid: test_job_type.uuid,
            job_arguments: HashMap::new(),
        };
        let test_schedule_item_serialized = serde_json::to_string(&test_schedule_item).unwrap();

        let mut headers = Headers::new();
        headers.set::<ContentType>(ContentType::json());
        let response = iron_error_translate(post(&*format!("http://{}/api/schedules", &*config::HTTP_BIND_ADDRESS), headers, &*test_schedule_item_serialized, &initialize_tests(store.replicate()?)))?;
        assert_eq!(response.status, Some(status::Ok));
        let body: PostResponse = parse_body(response.body)?;
        let new_job = store.get_job_schedule_item(body.uuid)?.unwrap();
        assert_eq!(new_job, ScheduleItem {
            uuid: body.uuid,
            interval: test_schedule_item.interval,
            job_type_uuid: test_schedule_item.job_type_uuid,
            job_arguments: test_schedule_item.job_arguments,
            last_scheduled_at: None,
            last_scheduled_by: None,
        });
        Ok(())
    }

    #[test]
    fn test_schedule_item_delete() -> Result<(), String> {
        let mut store: StoreRef = store::init_store_untyped();
        store.clean();
        let test_node_type = make_node_type(&mut store)?;
        store.set_node_type(test_node_type.uuid)?;
        let test_job_type = make_job_type(&mut store)?;
        let test_schedule = make_schedule_item(&mut store, test_job_type.uuid, None, None)?;

        let response = iron_error_translate(delete(&*format!("http://{}/api/schedules/{}", &*config::HTTP_BIND_ADDRESS, test_schedule.uuid.hyphenated()), Headers::new(), &initialize_tests(store.replicate()?)))?;
        assert_eq!(response.status, Some(status::Ok));
        assert_eq!(store.get_job_schedule_item(test_schedule.uuid)?, None);
        Ok(())
    }

}
