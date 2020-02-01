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
