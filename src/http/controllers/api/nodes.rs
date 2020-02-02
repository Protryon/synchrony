use crate::http::middleware::redis::IronRedis;
use iron::prelude::*;
use serde::{Deserialize, Serialize};
use crate::exec::node::Node;
use super::{ get_uuid_from_arg, redis_error_translate, option_translate };

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
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


#[cfg(test)]
mod tests {
    use super::*;
    use iron_test::request::get;
    use iron::Headers;
    use crate::http::controllers::tests::*;
    use crate::config;
    use iron::status;
    use crate::http::tests::initialize_tests;
    use crate::store::{ self, StoreRef, tests::* };

    #[test]
    fn test_nodes_index() -> Result<(), String> {
        let mut store: StoreRef = store::init_store_untyped();
        store.clean();
        let test_node_type = make_node_type(&mut store)?;
        store.set_node_type(test_node_type.uuid)?;
        let nodes = store.get_nodes()?;
        assert_eq!(nodes.len(), 1);
        let store_node = store.get_node().clone();

        let response = iron_error_translate(get(&*format!("http://{}/api/nodes", &*config::HTTP_BIND_ADDRESS), Headers::new(), &initialize_tests(store)))?;
        assert_eq!(response.status, Some(status::Ok));
        let body: IndexResponse = parse_body(response.body)?;
        assert_eq!(body, IndexResponse {
            nodes: vec![Node { node_type: None, ..store_node }],
        });
        Ok(())
    }

    #[test]
    fn test_nodes_get() -> Result<(), String> {
        let mut store: StoreRef = store::init_store_untyped();
        store.clean();
        let test_node_type = make_node_type(&mut store)?;
        store.set_node_type(test_node_type.uuid)?;
        let nodes = store.get_nodes()?;
        assert_eq!(nodes.len(), 1);
        let store_node = store.get_node().clone();

        let response = iron_error_translate(get(&*format!("http://{}/api/nodes/{}", &*config::HTTP_BIND_ADDRESS, store_node.uuid.hyphenated()), Headers::new(), &initialize_tests(store)))?;
        assert_eq!(response.status, Some(status::Ok));
        let body: Node = parse_body(response.body)?;
        assert_eq!(body, Node { node_type: None, ..store_node });
        Ok(())
    }

}
