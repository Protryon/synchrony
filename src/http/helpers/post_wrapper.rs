use super::control::status_error;
use iron::mime::*;
use iron::prelude::*;
use iron::status;
use iron::Handler;
use log::*;
use serde::{de, Serialize};
use std::fmt::Debug;

pub struct WrappedHandler<T: de::DeserializeOwned + 'static, K: Serialize + 'static> {
    handler: fn(&mut Request, &T) -> Result<K, IronResult<Response>>,
}

impl<T: de::DeserializeOwned + Clone + Debug + 'static, K: Serialize + 'static> Handler
    for WrappedHandler<T, K>
{
    fn handle(&self, req: &mut Request) -> IronResult<Response> {
        let maybe_body = req.get::<bodyparser::Struct<T>>();
        if maybe_body.is_err() {
            error!("decoding body failed: {:?}", maybe_body);
            return status_error(status::BadRequest);
        }
        let optionally_body = maybe_body.unwrap();
        if optionally_body.is_none() {
            return status_error(status::BadRequest);
        }
        let body = optionally_body.unwrap();
        let response = (self.handler)(req, &body);
        match response {
            Ok(data) => Ok(Response::with((
                Mime(TopLevel::Application, SubLevel::Json, vec![]),
                status::Ok,
                serde_json::to_string(&data).unwrap(),
            ))),
            Err(data) => data,
        }
    }
}

type HandleFunc<T, K> = fn(&mut Request, &T) -> Result<K, IronResult<Response>>;

pub fn json_wrap<T: de::DeserializeOwned + 'static, K: Serialize + 'static>(
    handler: HandleFunc<T, K>,
) -> WrappedHandler<T, K> {
    WrappedHandler { handler }
}

// NoDeserialize
pub fn serialize_wrap<K: Serialize + 'static>(handler: HandleFunc<(), K>) -> WrappedHandler<(), K> {
    WrappedHandler { handler }
}
