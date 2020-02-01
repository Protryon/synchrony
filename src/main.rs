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

mod util;
mod store;
mod exec;
mod scheduler;
mod threads;

use store::*;
use std::sync::{ Mutex, Arc };
use store::init_store;

fn main() {
    Builder::from_default_env()
        .filter_level(LevelFilter::Info)
        .init();
    let mut store = init_store();
    let thread_count = store.get_node().node_type.as_ref().unwrap().thread_count;
    info!("Started node '{}'", store.get_node().uuid.hyphenated().to_string());
    let store_mutex = Arc::new(Mutex::new(store));
    threads::ping_thread::start_thread(store_mutex.clone());
    threads::scheduler_thread::start_thread(store_mutex.clone());
    for _ in 0..thread_count {
        threads::worker_thread::start_thread(store_mutex.clone());
    }
    loop {
        std::thread::sleep_ms(1000);
    }
}
