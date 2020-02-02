use crate::http::middleware::redis::IronRedis;
use iron::prelude::*;
use serde::{Deserialize, Serialize};
use crate::exec::node_type::NodeType;
use iron::status;
use crate::http::helpers::control::status_error;
use log::*;
use super::{ get_uuid_from_arg, redis_error_translate, option_translate };

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct IndexResponse {
    node_types: Vec<NodeType>,
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
    let node_types = redis_error_translate(store.get_node_types())?;
    Ok(IndexResponse {
        node_types: node_types,
    })
}

pub fn get(
    req: &mut Request,
    _: &(),
) -> Result<NodeType, IronResult<Response>> {
    let uuid = get_uuid_from_arg(req, "uuid")?;
    let mut store = req.extensions
        .get::<IronRedis>()
        .unwrap()
        .lock()
        .unwrap();
    let node_type = option_translate(redis_error_translate(store.get_node_type(uuid))?)?;
    Ok(node_type)
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PostResponse {
    status: String,
}

pub fn post(
    req: &mut Request,
    body: &NodeType,
) -> Result<PostResponse, IronResult<Response>> {
    let uuid = get_uuid_from_arg(req, "uuid")?;
    if body.uuid != uuid {
        warn!("UUID in POST body and URL must match.");
        return Err(status_error(status::BadRequest));
    }
    let mut store = req.extensions
        .get::<IronRedis>()
        .unwrap()
        .lock()
        .unwrap();
    redis_error_translate(store.new_node_type(body))?;
    Ok(PostResponse {
        status: "ok".to_string(),
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
    use uuid::Uuid;

    #[test]
    fn test_node_types_index() -> Result<(), String> {
        let mut store: StoreRef = store::init_store_untyped();
        store.clean();
        let test_node_type = make_node_type(&mut store)?;
        store.set_node_type(test_node_type.uuid)?;

        let response = iron_error_translate(get(&*format!("http://{}/api/node_types", &*config::HTTP_BIND_ADDRESS), Headers::new(), &initialize_tests(store)))?;
        assert_eq!(response.status, Some(status::Ok));
        let body: IndexResponse = parse_body(response.body)?;
        assert_eq!(body, IndexResponse {
            node_types: vec![test_node_type],
        });
        Ok(())
    }

    #[test]
    fn test_node_types_get() -> Result<(), String> {
        let mut store: StoreRef = store::init_store_untyped();
        store.clean();
        let test_node_type = make_node_type(&mut store)?;
        store.set_node_type(test_node_type.uuid)?;
        let store_node = store.get_node().clone();

        let response = iron_error_translate(get(&*format!("http://{}/api/node_types/{}", &*config::HTTP_BIND_ADDRESS, store_node.node_type_uuid.unwrap().hyphenated()), Headers::new(), &initialize_tests(store)))?;
        assert_eq!(response.status, Some(status::Ok));
        let body: NodeType = parse_body(response.body)?;
        assert_eq!(body, store_node.node_type.unwrap());
        Ok(())
    }

    #[test]
    fn test_node_types_post() -> Result<(), String> {
        let mut store: StoreRef = store::init_store_untyped();
        store.clean();
        let test_node_type = NodeType {
            name: "test_node_type".to_string(),
            uuid: Uuid::new_v4(),
            thread_count: 1,
        };
        let node_type_serialized = serde_json::to_string(&test_node_type).unwrap();

        let mut headers = Headers::new();
        headers.set::<ContentType>(ContentType::json());
        let response = iron_error_translate(post(&*format!("http://{}/api/node_types/{}", &*config::HTTP_BIND_ADDRESS, test_node_type.uuid.hyphenated()), headers, &*node_type_serialized, &initialize_tests(store.replicate()?)))?;
        assert_eq!(response.status, Some(status::Ok));
        store.set_node_type(test_node_type.uuid)?;
        assert_eq!(store.get_node().node_type.as_ref().unwrap().clone(), test_node_type);
        Ok(())
    }

}
