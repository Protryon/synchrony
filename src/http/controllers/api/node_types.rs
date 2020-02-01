use crate::http::middleware::redis::IronRedis;
use iron::prelude::*;
use serde::{Deserialize, Serialize};
use crate::exec::node_type::NodeType;
use iron::status;
use crate::http::helpers::control::status_error;
use log::*;
use super::{ get_uuid_from_arg, redis_error_translate, option_translate };

#[derive(Debug, Clone, Serialize, Deserialize)]
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
