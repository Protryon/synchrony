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
use std::process::exit;

mod util;
mod store;
mod exec;
mod scheduler;
mod threads;

use util::config;
use store::*;
use std::sync::{ Mutex, Arc };

fn main() {
    Builder::from_default_env()
        .filter_level(LevelFilter::Info)
        .init();
    let mut store: StoreRef;
    if &*config::STORE_TYPE == "redis" {
        let redis_connected = store::redis::RedisStore::connect();
        match redis_connected {
            Err(e) => {
                error!("Error connecting to redis: {}", e);
                exit(1);
            }
            Ok(redis_store) => {
                store = Box::new(redis_store);
            }
        }
    } else {
        error!("Invalid STORE_TYPE configuration option: {}", &*config::STORE_TYPE);
        exit(1);
    }
    let node_types = store.get_node_types();
    if node_types.is_err() {
        error!("Failed to get node types from store: {}", node_types.err().unwrap());
        exit(1);
    }
    let our_node_type = node_types.as_ref().unwrap().iter().find(|item| item.name == *config::NODE_TYPE);
    if our_node_type.is_none() {
        error!("Invalid node type specified, not found: {}", *config::NODE_TYPE);
        exit(1);
    }
    let raw_node_type = our_node_type.unwrap();
    store.get_node().node_type_uuid = Some(raw_node_type.uuid);
    println!("{:?}", raw_node_type);
    info!("Started node '{}'", store.get_node().uuid.hyphenated().to_string());
    let store_mutex = Arc::new(Mutex::new(store));
    threads::ping_thread::start_thread(store_mutex.clone());
    threads::scheduler_thread::start_thread(store_mutex.clone());
    for _ in 0..raw_node_type.thread_count {
        threads::worker_thread::start_thread(store_mutex.clone());
    }
    loop {
        std::thread::sleep_ms(1000);
    }
}
