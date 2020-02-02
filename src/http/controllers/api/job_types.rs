use crate::http::middleware::redis::IronRedis;
use iron::prelude::*;
use serde::{Deserialize, Serialize};
use crate::exec::job_type::JobType;
use uuid::Uuid;
use std::collections::HashMap;
use serde_json::Value;
use super::{ get_uuid_from_arg, redis_error_translate, option_translate };

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
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
    fn test_job_types_index() -> Result<(), String> {
        let mut store: StoreRef = store::init_store_untyped();
        store.clean();
        let test_node_type = make_node_type(&mut store)?;
        store.set_node_type(test_node_type.uuid)?;
        let test_job_type = make_job_type(&mut store)?;

        let response = iron_error_translate(get(&*format!("http://{}/api/job_types", &*config::HTTP_BIND_ADDRESS), Headers::new(), &initialize_tests(store)))?;
        assert_eq!(response.status, Some(status::Ok));
        let body: IndexResponse = parse_body(response.body)?;
        assert_eq!(body, IndexResponse {
            job_types: vec![test_job_type],
        });
        Ok(())
    }

    #[test]
    fn test_job_types_get() -> Result<(), String> {
        let mut store: StoreRef = store::init_store_untyped();
        store.clean();
        let test_node_type = make_node_type(&mut store)?;
        store.set_node_type(test_node_type.uuid)?;
        let test_job_type = make_job_type(&mut store)?;

        let response = iron_error_translate(get(&*format!("http://{}/api/job_types/{}", &*config::HTTP_BIND_ADDRESS, test_job_type.uuid.hyphenated()), Headers::new(), &initialize_tests(store)))?;
        assert_eq!(response.status, Some(status::Ok));
        let body: JobType = parse_body(response.body)?;
        assert_eq!(body, test_job_type);
        Ok(())
    }

    #[test]
    fn test_job_types_post() -> Result<(), String> {
        let mut store: StoreRef = store::init_store_untyped();
        store.clean();
        let test_node_type = make_node_type(&mut store)?;
        store.set_node_type(test_node_type.uuid)?;

        let test_job_type = PostBody {
            executor: "bash".to_string(),
            name: "test".to_string(),
            node_type: "default".to_string(),
            timeout: None,
            unique: false,
            metadata: HashMap::new(),
        };
        let job_type_serialized = serde_json::to_string(&test_job_type).unwrap();

        let mut headers = Headers::new();
        headers.set::<ContentType>(ContentType::json());
        let response = iron_error_translate(post(&*format!("http://{}/api/job_types", &*config::HTTP_BIND_ADDRESS), headers, &*job_type_serialized, &initialize_tests(store.replicate()?)))?;
        assert_eq!(response.status, Some(status::Ok));
        let body: PostResponse = parse_body(response.body)?;
        let new_job = store.get_job_type(body.uuid)?.unwrap();
        assert_eq!(new_job, JobType {
            uuid: body.uuid,
            executor: test_job_type.executor,
            name: test_job_type.name,
            node_type: test_job_type.node_type,
            timeout: test_job_type.timeout,
            unique: test_job_type.unique,
            metadata: test_job_type.metadata,
        });
        Ok(())
    }

}
