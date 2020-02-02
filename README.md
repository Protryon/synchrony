# Synchrony
Synchrony is a distributed job management engine inspired in part by Sidekiq.

## Goals

* Provide a highly stable job management engine that can scale to millions of jobs/second.
* Provide a robust architecture allowing endless customization.
* Allow easy migration from other job management engines.

## Status

Synchrony is in an early beta stage. It is not ready for production environments, pending extensive testing.

## Building

Prerequisites:
* Rust 1.40+ Stable

1. Clone the synchrony repository:
```
$ git clone git@github.com:Protryon/synchrony.git
```

2. Within the `synchrony` directory, run `$ crate build`.
3. Run Synchrony with `$ ./target/debug/synchrony`.

## Running Synchrony

Synchrony is designed to be a minimally-local configuration in order to ease scaling pains.

Environment variables are used to provide parameters to connect to a given store (Redis is supported and selected by default), and tell the node what kind of node it is, and therefore what kinds of jobs it should process.

* `NODE_TYPE`: Default value is `default`. This value must match a name within the `node_types` hash key in Redis or equivalent.
* `STORE_TYPE`: Default value is `redis`. Currently only `redis` is supported.
* `REDIS_HOST`: Default value is `127.0.0.1`.
* `REDIS_PORT`: Default value is `6379`.
* `REDIS_DATABASE`: Default value is `<empty>`.

### Redis configuration

In order to get jobs flowing, you first must describe your jobs within Redis or equivalent.

There are 2 important keys that you must create in order to do this.

1. `node_types`: A hash, mapping a node type name to a JSON object describing it's configuration.
    ```
    {
        "name": "default",
        "uuid": "1bd2bf95-868a-4212-8a0a-82c7f442848e",
        "thread_count": 1
    }
    ```
    * `name`: Must match the name given in the redis hash key.
    * `uuid`: A random UUID to uniquely identify a node type.
    * `thread_count`: An integer specifying the maximum number of concurrent jobs nodes of this type may process.

Node types are essentially independent queues for job processing.

2. `job_types`: A hash, mapping a job type UUID to a JSON object describing it's configuration.
    ```
    {
        "uuid": "80d1c95a-a9cc-4dbb-a3fb-ff0f6b7d2c06",
        "name": "test_job",
        "executor": "bash",
        "metadata": {"command": "ls -l /"},
        "unique": false,
        "node_type": "default",
        "timeout": null
    }
    ```
    * `uuid`: A random UUID to uniquely identify a job type.
    * `name`: A human-useful name to describe the job type.
    * `executor`: A string enum value specifying the module needed to execute jobs of this type. See below.
    * `metadata`: A JSON object to be passed to the executor.
    * `unique`: Not yet implemented: A boolean value specifying that a given job type can have more than 1 active job across all nodes.
    * `node_type`: A reference to the name of a given node type that jobs of this type belong to.
    * `timeout`: Not yet implemented: A null or integer value specifying the maximum duration, in milliseconds, that this job can take before being forcefully terminated.

### Executors
Synchrony is built around the idea of end-use language agnosticism. Executors were created to support that idea, where a given job can have it's method of execution defined in a variety of ways.

* `bash`: Bash takes two parameters in `job_type`'s metadata, concatenates with the same two parameters from a given `job`'s arguments:
    1. `command`: Either a JSON array or a string representing the command. Required for `job_type`s, not for `job`s.
    2. `environment`: A JSON map specifying the environment variables to set when running the job. Never required.
* Custom Executors
    Synchrony only has builtin support for `bash` execution at the moment, which allows a wide range of integrations with other systems. Other, more specialized executors can be created by contributing to this project. Planned future builtin executors:
    * `sidekiq`
    * `http`

### Scheduling Jobs

Synchrony has the ability to automatically schedule jobs in a cron-like fashion.

Scheduled jobs can be easily managed directly through the store, or through the HTTP API. The interval (in milliseconds) specifies how often a job should run.

Note that there is no notion of catching up jobs if workers have been offline for some time, so do not rely on execution counts based on time.

### Running Jobs

Work in progress.

There are 2 ways to manually queue a job in Synchrony:
1. Use the HTTP API provided by all Synchrony nodes to create a new job.
2. Use a client implementation of the chosen store you are using, and add jobs directly to the relevant queue (i.e. `jobs_waiting_<node_type_uuid>`).

### Data formats

The following are standard formats used to represent various data types with Synchrony.

#### Job Type
```
{
    "uuid": "b30833c1-83b0-4dda-a439-97e3c97bbaa5",
    "name": "test_job_type",
    "executor": "bash",
    "metadata": { command: "echo 'test'" },
    "unique": false,
    "node_type": "default",
    "timeout": null
}
```

* `uuid`: Universally Unique ID
* `name`: Human readable name for convenience
* `executor`: String enum value for current executor
* `metadata`: Arguments to be used by the specified executor
* `unique`: If true, only one job can execute across the network at one time
* `node_type`: The type of nodes this job type can execute on
* `timeout`: `null` or a time in milliseconds specifying how long the executor should wait before killing the job

#### Job
```
{
    "uuid": "b30833c1-83b0-4dda-a439-97e3c97bbaa5",
    "job_type_uuid": "b30833c1-83b0-4dda-a439-97e3c97bbaa5",
    "arguments": {},
    "executing_node": "b30833c1-83b0-4dda-a439-97e3c97bbaa5",
    "enqueued_at": 1580651664039,
    "started_at": 1580651664039,
    "ended_at": 1580651664039,
    "results": { stdout: "test\n", stderr: "", exit_code: 0 },
    "errors": null
}
```

* `uuid`: Universally Unique ID
* `job_type_uuid`: UUID of job type accompying the job
* `arguments`: Arguments to be used by the specified executor within the job type
* `executing_node`: If already executing or finished, the node's UUID that is or has executed the job
* `enqueued_at`: At what time the job was created, milliseconds UNIX epoch
* `started_at`: At what time the job was started by a node, milliseconds UNIX epoch
* `ended_at`: At what time the job was finished by a node, milliseconds UNIX epoch
* `results`: An executor defined field upon job completion, or `null` if none provided
* `errors`: An executor defined field upon job completion, or `null` if none provided

#### Node Type
```
{
    "uuid": "b30833c1-83b0-4dda-a439-97e3c97bbaa5",
    "name": "default",
    "thread_count": 16
}
```

* `uuid`: Universally Unique ID
* `name`: Human readable name for convenience
* `thread_count`: An integer specifying how many concurrent jobs nodes of this type can process

#### Node
```
{
    "uuid": "b30833c1-83b0-4dda-a439-97e3c97bbaa5",
    "node_type_uuid": "b30833c1-83b0-4dda-a439-97e3c97bbaa5",
    "last_ping": 1580651664039
}
```

* `uuid`: Universally Unique ID
* `node_type_uuid`: UUID of node type accompying the node, decided by the node on startup
* `last_ping`: At what time the node last sent a ping to the store (by default every 5 seconds), milliseconds UNIX epoch

#### Schedule Item
```
{
    "uuid": "b30833c1-83b0-4dda-a439-97e3c97bbaa5",
    "interval": 60000,
    "last_scheduled_by": "b30833c1-83b0-4dda-a439-97e3c97bbaa5",
    "last_scheduled_at": 1580651664039,
    "job_type_uuid": "b30833c1-83b0-4dda-a439-97e3c97bbaa5",
    "job_arguments": {}
}
```

* `uuid`: Universally Unique ID
* `interval`: Minimum number of milliseconds between job runs
* `last_scheduled_by`: `null` if not previously run, or the uuid of the node that last scheduled (not run) this job
* `last_scheduled_at`: `null` if not previously run, or the time in milliseconds UNIX epoch when the job was last scheduled (not run)
* `job_type_uuid`: UUID of job type accompying the job
* `job_arguments`: Arguments to be passed to the job upon being enqueued

#### 

### HTTP API
* All requests are authenticated with an `Authorization: Bearer <API_KEY>` header, as specified via environment variable or defaulted to `dev_key`.
* All post requests must have `Content-Type: application/json`.
* All responses other than `GET /health` are JSON responses. `200 OK` status indicates success, anything else is failure.

#### GET /health
A simple endpoint that returns a `200 OK` response with payload of `ok`.

#### GET /api/job_types
Gets a list of all defined job types.

Response format:
```
{
    job_types: [
        <Job Type>
    ]
}
```

#### GET /api/job_types/:uuid
Gets a single job type.

Response format:
```
<Job Type>
```

#### POST /api/job_types
Creates a new job type.

Request format:
```
<Job Type without UUID>
```

Response format:
```
{
    status: "ok",
    uuid: "b30833c1-83b0-4dda-a439-97e3c97bbaa5"
}
```

#### GET /api/jobs/:node_type_uuid/queued
Gets all enqueued jobs for a given node type.

Response format:
```
{
    jobs: [
        <Job>
    ]
}
```

#### GET /api/jobs/:node_type_uuid/in_progress
Gets all currently executing jobs for a given node type.

Response format:
Same as `GET /api/jobs/:node_type_uuid/queued` above.

#### GET /api/jobs/:node_type_uuid/finished
Gets all finished jobs for a given node type.

Response format:
Same as `GET /api/jobs/:node_type_uuid/queued` above.

Note that `results` and `errors` are replaced with `true`/`false`. To get the full results or errors, get the specific job via `GET /api/jobs/:node_type_uuid/:uuid` below.

#### GET /api/jobs/:node_type_uuid/:uuid
Gets a finished job's extended data, including full results/errors.

Response format:
```
<Job>
```

#### POST /api/jobs
Enqueues a new job to be executed. Note that the node that receives this request is not necessarily the node that will execute it.

Request format:
```
{
    "job_type_uuid": "b30833c1-83b0-4dda-a439-97e3c97bbaa5",
    "arguments": {}
}
```

Response format:
```
{
    status: "ok",
    uuid: "b30833c1-83b0-4dda-a439-97e3c97bbaa5"
}
```

#### GET /api/node_types
Gets a list of all defined node types.

Response format:
```
{
    node_types: [
        <Node Type>
    ]
}
```

#### GET /api/node_types/:uuid
Gets a single node type.

Response format:
```
<Node Type>
```

#### POST /api/node_types/:uuid
Creates a new node type, or updates an existing one. Note that the UUID must remain unchanged.

Request format:
```
<Node Type>
```

Response format:
```
{
    "status": "ok"
}
```

#### GET /api/nodes
Gets a list of all nodes that have been active in the last 20 seconds.

Response format:
```
{
    nodes: [
        <Node>
    ]
}
```

#### GET /api/nodes/:uuid
Gets information on a specific node.

Response format:
```
<Node>
```

#### GET /api/schedules
Gets a list of all schedule items.

Response format:
```
{
    schedules: [
        <Schedule Item>
    ]
}
```

#### GET /api/schedules/:uuid
Gets a single schedule item.

Response format:
```
<Schedule Item>
```

#### POST /api/schedules
Creates a new schedule item.

Request format:
```
{
    "interval": 60000,
    "job_type_uuid": "b30833c1-83b0-4dda-a439-97e3c97bbaa5",
    "job_arguments": {}
}
```

Response format:
```
{
    status: "ok",
    uuid: "b30833c1-83b0-4dda-a439-97e3c97bbaa5"
}
```

#### DELETE /api/schedules/:uuid
Deletes a schedule item permanently.

Response format:
```
{
    status: "ok",
    uuid: "b30833c1-83b0-4dda-a439-97e3c97bbaa5"
}
```

## Future Work
* Create a watchdog thread that looks for jobs claimed by dead nodes and requeues them depending on job configuration.
* Build out independent frontend that interfaces with the HTTP API.
* Create logging client to export logs from finished jobs in Redis.
* Create `http` executor.
* Create client language implementations for direct communication.
