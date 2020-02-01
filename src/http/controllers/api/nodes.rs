use crate::http::middleware::redis::IronRedis;
use iron::prelude::*;
use serde::{Deserialize, Serialize};
use crate::exec::node::Node;
use super::{ get_uuid_from_arg, redis_error_translate, option_translate };

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IndexResponse {
    nodes: Vec<Node>,
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
    let nodes = redis_error_translate(store.get_nodes())?;
    Ok(IndexResponse {
        nodes: nodes,
    })
}

pub fn get(
    req: &mut Request,
    _: &(),
) -> Result<Node, IronResult<Response>> {
    let uuid = get_uuid_from_arg(req, "uuid")?;
    let mut store = req.extensions
        .get::<IronRedis>()
        .unwrap()
        .lock()
        .unwrap();
    let node = option_translate(redis_error_translate(store.get_other_node(uuid))?)?;
    Ok(node)
}
