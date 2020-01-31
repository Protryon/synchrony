
use std::thread;
use crate::StoreRef;
use std::sync::{ Mutex, Arc };
use log::*;
use crate::util::time::epoch;
use crate::exec::job::Job;
use uuid::Uuid;

pub fn start_thread(store: Arc<Mutex<StoreRef>>) {
    thread::spawn(move || {
        let interval = store.lock().unwrap().get_ping_interval_ms();
        loop {
            thread::sleep_ms(interval);

            let job_schedule = store.lock().unwrap().get_job_schedule();
            if job_schedule.is_err() {
                error!("Error getting job schedule from redis server: {}", job_schedule.err().unwrap());
                continue;
            }
            let eval_time = epoch();
            for schedule_item in job_schedule.unwrap() {
                if schedule_item.last_scheduled_at.is_none() || eval_time - schedule_item.last_scheduled_at.unwrap() < schedule_item.interval {
                    let mut guarded_store = store.lock().unwrap();
                    let claim_result = guarded_store.claim_job_scheduled(&schedule_item);
                    match claim_result {
                        Err(e) => { error!("Error claiming job schedule from redis server: {}", e); },
                        Ok(None) => {},
                        Ok(Some(_)) => {
                            let job_type = guarded_store.get_new_job_type(schedule_item.job_type_uuid);
                            if job_type.is_err() || job_type.as_ref().unwrap().is_none() {
                                error!("Error getting job type from redis server: {}", job_type.err().unwrap());
                                continue;
                            }
                            let enqueue_result = guarded_store.enqueue_job(Job {
                                uuid: Uuid::new_v4(),
                                job_type_uuid: schedule_item.job_type_uuid,
                                job_type: Some(job_type.unwrap().unwrap()),
                                arguments: schedule_item.job_arguments,
                                executing_node: None,
                                enqueued_at: None,
                                started_at: None,
                                ended_at: None,
                                results: None,
                                errors: None,
                            });
                            if enqueue_result.is_err() {
                                error!("Error enqueuing job from redis server: {}", enqueue_result.err().unwrap());
                                continue;
                            }
                        },
                    };
                }
            }
        }
    });
}
