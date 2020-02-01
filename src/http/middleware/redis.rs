use iron::prelude::*;
use std::sync::{ Arc, Mutex };
use crate::store::StoreRef;

pub struct IronRedis {
    pub binder: Arc<Mutex<StoreRef>>,
}

impl iron::typemap::Key for IronRedis {
    type Value = Arc<Mutex<StoreRef>>;
}

impl iron::BeforeMiddleware for IronRedis {
    fn before(&self, req: &mut Request) -> IronResult<()> {
        req.extensions.insert::<IronRedis>(self.binder.clone());
        Ok(())
    }
}
