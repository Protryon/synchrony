
use std::thread;
use crate::StoreRef;
use log::*;
use crate::exec::executors::*;
use crate::exec::executor::*;
use serde_json::Value;

fn run_loop(store: &mut StoreRef) {
    let dequeued_item = store.dequeue_job();
    if dequeued_item.is_err() {
        error!("Error getting job schedule from redis server: {}", dequeued_item.err().unwrap());
        return;
    }
    let job = dequeued_item.unwrap();
    let job_type = job.job_type.as_ref().unwrap();
    info!("Starting job '{}' of type '{}' / '{}'", job.uuid.hyphenated(), job_type.name, job.job_type_uuid.hyphenated());
    run_job(store, job);
}

pub fn start_thread(mut store: StoreRef) {
    thread::spawn(move || {
        loop {
            run_loop(&mut store);
        }
    });
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::store::tests::*;
    use crate::store::init_store_untyped;
    use serde_json::{ Map, Number };

    #[test]
    fn can_execute_job() -> Result<(), String> {
        let mut store = init_store_untyped();
        store.clean();
        let test_node_type = make_node_type(&mut store)?;
        store.set_node_type(test_node_type.uuid)?;
        let test_job_type = make_job_type(&mut store)?;
        let mut test_job = make_job(&mut store, &test_job_type)?;
        run_loop(&mut store);
        let finished_jobs = store.get_all_jobs_finished()?;
        assert_eq!(finished_jobs.len(), 1);
        test_job.enqueued_at = finished_jobs[0].enqueued_at;
        test_job.started_at = finished_jobs[0].started_at;
        test_job.executing_node = finished_jobs[0].executing_node;
        test_job.ended_at = finished_jobs[0].ended_at;
        let mut output = Map::new();
        output.insert("stdout".to_string(), Value::String("test\n".to_string()));
        output.insert("stderr".to_string(), Value::String("".to_string()));
        output.insert("exit_code".to_string(), Value::Number(Number::from(0)));
        test_job.results = Some(Value::Object(output));
        assert_eq!(finished_jobs[0], test_job);
        Ok(())
    }
}
