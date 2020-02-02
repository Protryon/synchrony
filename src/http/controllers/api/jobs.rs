
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

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
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
    let current_node_type_uuid = store.get_node().node_type_uuid.unwrap();
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
    let current_node_type_uuid = store.get_node().node_type_uuid.unwrap();
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
    let current_node_type_uuid = store.get_node().node_type_uuid.unwrap();
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
    let current_node_type_uuid = store.get_node().node_type_uuid.unwrap();
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


#[cfg(test)]
mod tests {
    use super::*;
    use iron_test::request::{ post, get };
    use iron::{ Headers, headers::ContentType };
    use crate::http::controllers::tests::*;
    use crate::config;
    use iron::status;
    use crate::http::tests::initialize_tests;
    use crate::store::{ self, StoreRef, tests::* };

    #[test]
    fn test_jobs_index_queued() -> Result<(), String> {
        let mut store: StoreRef = store::init_store_untyped();
        store.clean();
        let test_node_type = make_node_type(&mut store)?;
        store.set_node_type(test_node_type.uuid)?;
        let test_job_type = make_job_type(&mut store)?;
        let mut test_job = make_job(&mut store, &test_job_type)?;
        let response = iron_error_translate(get(&*format!("http://{}/api/jobs/{}/queued", &*config::HTTP_BIND_ADDRESS, test_node_type.uuid.hyphenated()), Headers::new(), &initialize_tests(store.replicate()?)))?;
        assert_eq!(response.status, Some(status::Ok));
        let body: IndexResponse = parse_body(response.body)?;
        test_job.job_type = None;
        assert_eq!(body.jobs.len(), 1);
        test_job.enqueued_at = body.jobs[0].enqueued_at;
        assert_eq!(body, IndexResponse {
            jobs: vec![test_job],
        });
        assert_eq!(store.get_node().node_type_uuid.unwrap(), test_node_type.uuid);
        Ok(())
    }

    #[test]
    fn test_jobs_index_in_progress() -> Result<(), String> {
        let mut store: StoreRef = store::init_store_untyped();
        store.clean();
        let test_node_type = make_node_type(&mut store)?;
        store.set_node_type(test_node_type.uuid)?;
        let test_job_type = make_job_type(&mut store)?;
        let mut test_job = make_job(&mut store, &test_job_type)?;
        store.dequeue_job()?;
        let response = iron_error_translate(get(&*format!("http://{}/api/jobs/{}/in_progress", &*config::HTTP_BIND_ADDRESS, test_node_type.uuid.hyphenated()), Headers::new(), &initialize_tests(store.replicate()?)))?;
        assert_eq!(response.status, Some(status::Ok));
        let body: IndexResponse = parse_body(response.body)?;
        test_job.job_type = None;
        assert_eq!(body.jobs.len(), 1);
        test_job.enqueued_at = body.jobs[0].enqueued_at;
        test_job.started_at = body.jobs[0].started_at;
        test_job.executing_node = body.jobs[0].executing_node;
        assert_eq!(body, IndexResponse {
            jobs: vec![test_job],
        });
        assert_eq!(store.get_node().node_type_uuid.unwrap(), test_node_type.uuid);
        Ok(())
    }

    #[test]
    fn test_jobs_index_finished() -> Result<(), String> {
        let mut store: StoreRef = store::init_store_untyped();
        store.clean();
        let test_node_type = make_node_type(&mut store)?;
        store.set_node_type(test_node_type.uuid)?;
        let test_job_type = make_job_type(&mut store)?;
        let mut test_job = make_job(&mut store, &test_job_type)?;
        store.dequeue_job()?;
        store.finish_job(test_job.clone(), Some(Value::String("output".to_string())), Some(Value::String("errors".to_string())))?;

        let response = iron_error_translate(get(&*format!("http://{}/api/jobs/{}/finished", &*config::HTTP_BIND_ADDRESS, test_node_type.uuid.hyphenated()), Headers::new(), &initialize_tests(store.replicate()?)))?;
        assert_eq!(response.status, Some(status::Ok));
        let body: IndexResponse = parse_body(response.body)?;
        test_job.job_type = None;
        assert_eq!(body.jobs.len(), 1);
        test_job.enqueued_at = body.jobs[0].enqueued_at;
        test_job.started_at = body.jobs[0].started_at;
        test_job.ended_at = body.jobs[0].ended_at;
        test_job.executing_node = body.jobs[0].executing_node;
        test_job.results = Some(Value::Bool(true));
        test_job.errors = Some(Value::Bool(true));
        assert_eq!(body, IndexResponse {
            jobs: vec![test_job],
        });
        assert_eq!(store.get_node().node_type_uuid.unwrap(), test_node_type.uuid);
        Ok(())
    }

    #[test]
    fn test_jobs_get() -> Result<(), String> {
        let mut store: StoreRef = store::init_store_untyped();
        store.clean();
        let test_node_type = make_node_type(&mut store)?;
        store.set_node_type(test_node_type.uuid)?;
        let test_job_type = make_job_type(&mut store)?;
        let mut test_job = make_job(&mut store, &test_job_type)?;
        store.dequeue_job()?;
        store.finish_job(test_job.clone(), Some(Value::String("output".to_string())), Some(Value::String("errors".to_string())))?;

        let response = iron_error_translate(get(&*format!("http://{}/api/jobs/{}/{}", &*config::HTTP_BIND_ADDRESS, test_node_type.uuid.hyphenated(), test_job.uuid.hyphenated()), Headers::new(), &initialize_tests(store)))?;
        assert_eq!(response.status, Some(status::Ok));
        let body: Job = parse_body(response.body)?;
        test_job.job_type = None;
        test_job.enqueued_at = body.enqueued_at;
        test_job.started_at = body.started_at;
        test_job.ended_at = body.ended_at;
        test_job.executing_node = body.executing_node;
        test_job.results = Some(Value::String("output".to_string()));
        test_job.errors = Some(Value::String("errors".to_string()));
        assert_eq!(body, test_job);
        Ok(())
    }

    #[test]
    fn test_jobs_post() -> Result<(), String> {
        let mut store: StoreRef = store::init_store_untyped();
        store.clean();
        let test_node_type = make_node_type(&mut store)?;
        store.set_node_type(test_node_type.uuid)?;
        let test_job_type = make_job_type(&mut store)?;

        let test_job = PostBody {
            job_type_uuid: test_job_type.uuid,
            arguments: HashMap::new(),
        };
        let test_job_serialized = serde_json::to_string(&test_job).unwrap();

        let mut headers = Headers::new();
        headers.set::<ContentType>(ContentType::json());
        let response = iron_error_translate(post(&*format!("http://{}/api/jobs", &*config::HTTP_BIND_ADDRESS), headers, &*test_job_serialized, &initialize_tests(store.replicate()?)))?;
        assert_eq!(response.status, Some(status::Ok));
        let body: PostResponse = parse_body(response.body)?;
        let new_job = store.dequeue_job()?;
        assert_eq!(new_job.uuid, body.uuid);
        assert_eq!(new_job.job_type_uuid, test_job_type.uuid);
        assert_eq!(new_job.arguments, test_job.arguments);
        Ok(())
    }

}
