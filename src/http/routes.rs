use super::controllers::*;
use super::helpers::post_wrapper::*;
use router::Router;

pub fn add_routes(router: &mut Router) {
    router.get("/api/node_types", serialize_wrap(api::node_types::index), "node_types#index");
    router.get("/api/node_types/:uuid", serialize_wrap(api::node_types::get), "node_types#get");
    router.post("/api/node_types/:uuid", json_wrap(api::node_types::post), "node_types#post");

    router.get("/api/nodes", serialize_wrap(api::nodes::index), "nodes#index");
    router.get("/api/nodes/:uuid", serialize_wrap(api::nodes::get), "nodes#get");

    router.get("/api/job_types", serialize_wrap(api::job_types::index), "job_types#index");
    router.get("/api/job_types/:uuid", serialize_wrap(api::job_types::get), "job_types#get");
    router.post("/api/job_types", json_wrap(api::job_types::post), "job_types#post");

    router.get("/api/jobs/:node_type_uuid/queued", serialize_wrap(api::jobs::index_queued), "jobs#index_queued");
    router.get("/api/jobs/:node_type_uuid/in_progress", serialize_wrap(api::jobs::index_in_progress), "jobs#index_in_progress");
    router.get("/api/jobs/:node_type_uuid/finished", serialize_wrap(api::jobs::index_finished), "jobs#index_finished");
    router.get("/api/jobs/:node_type_uuid/:uuid", serialize_wrap(api::jobs::get), "jobs#get"); // gets only finished jobs, but includes all results/errors, not a boolean presence summary
    router.post("/api/jobs", json_wrap(api::jobs::post), "jobs#post");

    router.get("/api/schedules", serialize_wrap(api::schedule::index), "schedule#index");
    router.get("/api/schedules/:uuid", serialize_wrap(api::schedule::get), "schedule#get");
    router.delete("/api/schedules/:uuid", serialize_wrap(api::schedule::delete), "schedule#delete");
    router.post("/api/schedules", json_wrap(api::schedule::post), "schedule#post");

    router.get("/health", health::handle, "health");
}
