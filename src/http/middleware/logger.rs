use iron::prelude::*;
use iron::status;
use log::*;
use crate::util::time::epoch_us;

pub struct Logger;

impl iron::typemap::Key for Logger {
    type Value = u64;
}

impl iron::BeforeMiddleware for Logger {
    fn before(&self, req: &mut Request) -> IronResult<()> {
        req.extensions.insert::<Logger>(epoch_us());
        Ok(())
    }
}

impl iron::AfterMiddleware for Logger {
    fn after(&self, req: &mut Request, res: Response) -> IronResult<Response> {
        let delta = epoch_us() - *req.extensions.get::<Logger>().unwrap();
        info!(
            "'{}': {} /{}, got {}, took: {} ms",
            req.remote_addr.ip(),
            req.method.as_ref(),
            req.url.path().join("/"),
            res.status.unwrap_or(status::Ok),
            (delta as f64) / 1000.0
        );
        Ok(res)
    }

    fn catch(&self, req: &mut Request, err: IronError) -> IronResult<Response> {
        let delta = epoch_us() - *req.extensions.get::<Logger>().unwrap();
        warn!(
            "'{}': {} /{}, got {}, took: {} ms, error: {}",
            req.remote_addr.ip(),
            req.method.as_ref(),
            req.url.path().join("/"),
            err.response.status.unwrap_or(status::Ok),
            (delta as f64) / 1000.0,
            err.error
        );
        Ok(err.response) // avoid iron's default log
    }
}
