mod api_auth;
mod assert_json;
mod logger;
pub mod redis;

use iron::prelude::*;
use persistent::Read;
use std::sync::{ Arc, Mutex };
use crate::store::StoreRef;

pub fn add_middleware(chain: &mut Chain, store: Arc<Mutex<StoreRef>>) {
    chain.link_before(logger::Logger);
    chain.link_before(api_auth::ApiAuth);
    chain.link_before(Read::<bodyparser::MaxBodyLength>::one(1024 * 1024 * 50));
    chain.link_before(assert_json::AssertJson);
    chain.link_before(redis::IronRedis { binder: store });
    chain.link_after(logger::Logger);
}

#[cfg(test)]
pub mod tests {
    use super::*;

    pub fn add_middleware_test(chain: &mut Chain, store: Arc<Mutex<StoreRef>>) {
        chain.link_before(logger::Logger);
        chain.link_before(Read::<bodyparser::MaxBodyLength>::one(1024 * 1024 * 50));
        chain.link_before(assert_json::AssertJson);
        chain.link_before(redis::IronRedis { binder: store });
        chain.link_after(logger::Logger);
    }
}