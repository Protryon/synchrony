
use std::thread;
use crate::StoreRef;
use log::*;

pub fn start_thread(mut store: StoreRef) {
    thread::spawn(move || {
        let interval = store.get_ping_interval_ms();
        loop {
            let ping_result = store.ping();
            if ping_result.is_err() {
                error!("Error pinging redis server: {}", ping_result.err().unwrap());
            }
            thread::sleep_ms(interval);
        }
    });
}
