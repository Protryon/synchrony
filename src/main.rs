#[macro_use]
extern crate lazy_static;
extern crate log;
extern crate serde;
extern crate uuid;
extern crate redis;
extern crate time;

use env_logger::Builder;
use log::LevelFilter;
use log::*;
use std::thread;
use util::config;

mod util;
mod store;
mod exec;
mod scheduler;
mod threads;
mod http;

use store::*;
use store::init_store;

fn main() {
    Builder::from_default_env()
        .filter_level(LevelFilter::Info)
        .filter_module("hyper::server", LevelFilter::Warn)
        .init();
    let mut store = init_store();
    let thread_count = store.get_node().node_type.as_ref().unwrap().thread_count;
    info!("Started node '{}'", store.get_node().uuid.hyphenated().to_string());
    threads::ping_thread::start_thread(store.replicate().expect("failed to reconnect to redis"));
    threads::scheduler_thread::start_thread(store.replicate().expect("failed to reconnect to redis"));
    for _ in 0..thread_count {
        threads::worker_thread::start_thread(store.replicate().expect("failed to reconnect to redis"));
    }

    if &*config::HTTP_SERVER_ENABLED == "true" {
        http::start_thread(store.replicate().expect("failed to reconnect to redis"));
    }

    loop {
        thread::sleep_ms(1000);
    }
}
