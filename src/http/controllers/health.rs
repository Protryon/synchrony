use iron::mime::*;
use iron::prelude::*;
use iron::status;

pub fn handle(_: &mut Request) -> IronResult<Response> {
    Ok(Response::with((
        Mime(TopLevel::Text, SubLevel::Plain, vec![]),
        status::Ok,
        "ok",
    )))
}

#[cfg(test)]
mod tests {
    use super::*;
    use iron_test::request::get;
    use iron::Headers;
    use super::super::tests::stringify_body;
    use crate::config;

    const ROUTE: &str = "/health";

    #[test]
    fn test_can_get_health() -> Result<(), IronError> {
        let response = get(&*format!("http://{}{}", &*config::HTTP_BIND_ADDRESS, ROUTE), Headers::new(), &super::handle)?;
        assert_eq!(response.status, Some(status::Ok));
        let body = stringify_body(response.body);
        assert_eq!(body, "ok");
        Ok(())
    }

}
