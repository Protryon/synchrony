use super::super::helpers::control::status_error;
use crate::util::config;
use iron::headers::{Authorization, Bearer};
use iron::prelude::*;
use iron::status;

pub struct ApiAuth;

impl iron::BeforeMiddleware for ApiAuth {
    fn before(&self, req: &mut Request) -> IronResult<()> {
        let maybe_auth = req.headers.get::<Authorization<Bearer>>();
        if maybe_auth.is_none() {
            return status_error(status::Unauthorized);
        }
        let auth = maybe_auth.unwrap();
        if (*auth).token != *config::HTTP_API_KEY {
            return status_error(status::Unauthorized);
        }
        return Ok(());
    }
}
