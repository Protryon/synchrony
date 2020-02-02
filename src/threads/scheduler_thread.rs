
use std::thread;
use crate::StoreRef;
use log::*;
use crate::util::time::epoch;
use crate::exec::job::Job;
use uuid::Uuid;

fn run_loop(store: &mut StoreRef) {
    let job_schedule = store.get_job_schedule();
    if job_schedule.is_err() {
        error!("Error getting job schedule from redis server: {}", job_schedule.err().unwrap());
        return;
    }
    let eval_time = epoch();
    for schedule_item in job_schedule.unwrap() {
        let current_chunk = eval_time / schedule_item.interval;
        if schedule_item.last_scheduled_at.is_none() || current_chunk > (schedule_item.last_scheduled_at.unwrap() / schedule_item.interval) {
            let claim_result = store.claim_job_scheduled(&schedule_item);
            match claim_result {
                Err(e) => { error!("Error claiming job schedule from redis server: {}", e); },
                Ok(None) => {},
                Ok(Some(_)) => {
                    let job_type = store.get_job_type(schedule_item.job_type_uuid);
                    if job_type.is_err() || job_type.as_ref().unwrap().is_none() {
                        error!("Error getting job type from redis server: {}", job_type.err().unwrap());
                        return;
                    }
                    let enqueue_result = store.enqueue_job(Job {
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

pub fn start_thread(mut store: StoreRef) {
    thread::spawn(move || {
        let interval = store.get_ping_interval_ms();
        loop {
            thread::sleep_ms(interval);

            run_loop(&mut store);
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
        run_loop(&mut store);
        let queued_jobs = store.get_all_jobs_waiting()?;
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
        run_loop(&mut store);
        let queued_jobs = store.get_all_jobs_waiting()?;
        assert_eq!(queued_jobs.len(), 1);
        assert_eq!(queued_jobs[0].job_type_uuid, test_job_type.uuid);
        Ok(())
    }

    #[test]
    fn will_not_schedule_when_not_expired() -> Result<(), String> {
        let mut store = init_store_untyped();
        store.clean();
        let test_node_type = make_node_type(&mut store)?;
        store.set_node_type(test_node_type.uuid)?;
        let test_job_type = make_job_type(&mut store)?;
        let node_uuid = store.get_node().uuid;
        // assumption: the following 3 lines execute in < 500 ms (the default interval for make_schedule_item)
        make_schedule_item(&mut store, test_job_type.uuid, Some(node_uuid), Some(epoch() + 100))?;
        run_loop(&mut store);
        let mut queued_jobs = store.get_all_jobs_waiting()?;
        assert_eq!(queued_jobs.len(), 0);

        thread::sleep_ms(1000);
        run_loop(&mut store);
        queued_jobs = store.get_all_jobs_waiting()?;
        assert_eq!(queued_jobs.len(), 1);
        assert_eq!(queued_jobs[0].job_type_uuid, test_job_type.uuid);
        Ok(())
    }
}
