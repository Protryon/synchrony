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

Work in Progress

### Running Jobs

Work in progress.

There are 2 ways to manually queue a job in Synchrony:
1. Use the HTTP API provided by all Synchrony nodes to create a new job.
    Not yet implemented.
2. Use a client implementation of the chosen store you are using, and add jobs directly to the relevant queue (i.e. `jobs_waiting_<node_type_uuid>`).

## Future Work
* Build out documentation on scheduling jobs, HTTP API.
* Create `sidekiq` and `http` executors.
* Create client language implementations for direct communication.
* Build out indirect Redis configuration client.
* Create logging client to export logs from finished jobs in Redis.