mod routes;
mod middleware;
mod helpers;
mod controllers;

use router::Router;
use iron::prelude::*;
use std::thread;
use crate::util::config;
use crate::store::StoreRef;
use log::*;
use std::sync::{ Arc, Mutex };

pub fn initialize(store: StoreRef) {
    // routes
    let mut router = Router::new();
    routes::add_routes(&mut router);

    // middleware
    let mut chain = Chain::new(router);
    middleware::add_middleware(&mut chain, Arc::new(Mutex::new(store)));

    info!("Listening on {}!", &*config::HTTP_BIND_ADDRESS);
    Iron::new(chain).http(&*config::HTTP_BIND_ADDRESS).unwrap();
}


pub fn start_thread(store: StoreRef) {
    thread::spawn(move || {
        initialize(store);
    });
}

#[cfg(test)]
pub mod tests {
    use super::*;
    use iron::Handler;

    pub fn initialize_tests(store: StoreRef) -> Box<dyn Handler> {
        // routes
        let mut router = Router::new();
        routes::add_routes(&mut router);
    
        // middleware
        let mut chain = Chain::new(router);
        middleware::tests::add_middleware_test(&mut chain, Arc::new(Mutex::new(store)));
        Box::new(chain)
    }
}