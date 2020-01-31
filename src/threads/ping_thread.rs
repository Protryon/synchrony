
use std::thread;
use crate::StoreRef;
use std::sync::{ Mutex, Arc };
use log::*;

pub fn start_thread(store: Arc<Mutex<StoreRef>>) {
    thread::spawn(move || {
        let interval = store.lock().unwrap().get_ping_interval_ms();
        loop {
            let ping_result = store.lock().unwrap().ping();
            if ping_result.is_err() {
                error!("Error pinging redis server: {}", ping_result.err().unwrap());
            }
            thread::sleep_ms(interval);
        }
    });
}
