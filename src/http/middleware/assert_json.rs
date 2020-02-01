use super::super::helpers::control::status_error;
use iron::headers::ContentType;
use iron::method::Method;
use iron::prelude::*;
use iron::status;

pub struct AssertJson;

impl iron::BeforeMiddleware for AssertJson {
    fn before(&self, req: &mut Request) -> IronResult<()> {
        if req.method == Method::Post
            && !req
                .headers
                .get::<ContentType>()
                .map(|t| *t == ContentType::json())
                .unwrap_or(false)
        {
            status_error(status::UnsupportedMediaType)
        } else {
            Ok(())
        }
    }
}
