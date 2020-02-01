
use std::thread;
use crate::StoreRef;
use std::sync::{ Mutex, Arc };
use log::*;
use crate::util::time::epoch;
use crate::exec::job::Job;
use uuid::Uuid;

fn run_loop(store: &Arc<Mutex<StoreRef>>) {
    let job_schedule = store.lock().unwrap().get_job_schedule();
    if job_schedule.is_err() {
        error!("Error getting job schedule from redis server: {}", job_schedule.err().unwrap());
        return;
    }
    let eval_time = epoch();
    for schedule_item in job_schedule.unwrap() {
        if schedule_item.last_scheduled_at.is_none() || eval_time - schedule_item.last_scheduled_at.unwrap() >= schedule_item.interval {
            let mut guarded_store = store.lock().unwrap();
            let claim_result = guarded_store.claim_job_scheduled(&schedule_item);
            match claim_result {
                Err(e) => { error!("Error claiming job schedule from redis server: {}", e); },
                Ok(None) => {},
                Ok(Some(_)) => {
                    let job_type = guarded_store.get_job_type(schedule_item.job_type_uuid);
                    if job_type.is_err() || job_type.as_ref().unwrap().is_none() {
                        error!("Error getting job type from redis server: {}", job_type.err().unwrap());
                        return;
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
                        return;
                    }
                },
            };
        }
    }
}

pub fn start_thread(store: Arc<Mutex<StoreRef>>) {
    thread::spawn(move || {
        let interval = store.lock().unwrap().get_ping_interval_ms();
        loop {
            thread::sleep_ms(interval);

            run_loop(&store);
        }
    });
}


#[cfg(test)]
mod tests {
    use super::*;
    use crate::store::tests::*;
    use crate::store::init_store_untyped;

    #[test]
    fn can_schedule_first_time() -> Result<(), String> {
        let mut store = init_store_untyped();
        store.clean();
        let test_node_type = make_node_type(&mut store)?;
        store.set_node_type(test_node_type.uuid)?;
        let test_job_type = make_job_type(&mut store)?;
        make_schedule_item(&mut store, test_job_type.uuid, None, None)?;
        let store_ref = Arc::new(Mutex::new(store));
        run_loop(&store_ref);
        let mut store_guarded = store_ref.lock().unwrap();
        let queued_jobs = store_guarded.get_all_jobs_waiting()?;
        assert_eq!(queued_jobs.len(), 1);
        assert_eq!(queued_jobs[0].job_type_uuid, test_job_type.uuid);
        Ok(())
    }

    #[test]
    fn can_schedule_when_expired() -> Result<(), String> {
        let mut store = init_store_untyped();
        store.clean();
        let test_node_type = make_node_type(&mut store)?;
        store.set_node_type(test_node_type.uuid)?;
        let test_job_type = make_job_type(&mut store)?;
        let node_uuid = store.get_node().uuid;
        make_schedule_item(&mut store, test_job_type.uuid, Some(node_uuid), Some(0))?;
        let store_ref = Arc::new(Mutex::new(store));
        run_loop(&store_ref);
        let mut store_guarded = store_ref.lock().unwrap();
        let queued_jobs = store_guarded.get_all_jobs_waiting()?;
        assert_eq!(queued_jobs.len(), 1);
        assert_eq!(queued_jobs[0].job_type_uuid, test_job_type.uuid);
        Ok(())
    }

    fn get_all_jobs_waiting(store_ref: &Arc<Mutex<StoreRef>>) -> Result<Vec<Job>, String> {
        let mut store_guarded = store_ref.lock().unwrap();
        let queued_jobs = store_guarded.get_all_jobs_waiting()?;
        return Ok(queued_jobs);
    }

    #[test]
    fn will_not_schedule_when_not_expired() -> Result<(), String> {
        let mut store = init_store_untyped();
        store.clean();
        let test_node_type = make_node_type(&mut store)?;
        store.set_node_type(test_node_type.uuid)?;
        let test_job_type = make_job_type(&mut store)?;
        let node_uuid = store.get_node().uuid;
        // assumption: the following 3 lines execute in < 1 sec (the default interval for make_schedule_item)
        make_schedule_item(&mut store, test_job_type.uuid, Some(node_uuid), Some(epoch()))?;
        let store_ref = Arc::new(Mutex::new(store));
        run_loop(&store_ref);
        let mut queued_jobs = get_all_jobs_waiting(&store_ref)?;
        assert_eq!(queued_jobs.len(), 0);

        thread::sleep_ms(1000);
        run_loop(&store_ref);
        queued_jobs = get_all_jobs_waiting(&store_ref)?;
        assert_eq!(queued_jobs.len(), 1);
        assert_eq!(queued_jobs[0].job_type_uuid, test_job_type.uuid);
        Ok(())
    }
}
