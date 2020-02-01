pub mod job_types;
pub mod jobs;
pub mod node_types;
pub mod nodes;
pub mod schedule;

use router::Router;
use crate::http::helpers::control::status_error;
use iron::prelude::*;
use iron::status;
use uuid::Uuid;
use log::*;

pub fn get_uuid_from_arg(req: &Request, key: &str) -> Result<Uuid, IronResult<Response>> {
    let uuid_str = req.extensions.get::<Router>().unwrap().find(key);
    if uuid_str.is_none() {
        return Err(status_error(status::BadRequest));
    }
    let uuid = Uuid::parse_str(uuid_str.unwrap());
    if uuid.is_err() {
        warn!("Invalid UUID: {}", uuid_str.unwrap());
        return Err(status_error(status::BadRequest));
    }
    return Ok(uuid.unwrap());
}

pub fn redis_error_translate<T>(result: Result<T, String>) -> Result<T, IronResult<Response>> {
    match result {
        Err(e) => {
            error!("Redis server failed during API request: {}", e);
            Err(status_error(status::InternalServerError))
        },
        Ok(value) => {
            Ok(value)
        },
    }
}

pub fn option_translate<T>(option: Option<T>) -> Result<T, IronResult<Response>> {
    match option {
        None => Err(status_error(status::NotFound)),
        Some(value) => Ok(value),
    }
}