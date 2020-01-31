use std::env;

fn default_env(name: &str, default: &str) -> String {
    env::var(name).unwrap_or(default.to_string())
}

lazy_static! {
    pub static ref NODE_TYPE: String = { default_env("NODE_TYPE", "default") };
    pub static ref STORE_TYPE: String = { default_env("STORE_TYPE", "redis") };
    pub static ref REDIS_HOST: String = { default_env("REDIS_HOST", "127.0.0.1") };
    pub static ref REDIS_PORT: String = { default_env("REDIS_PORT", "6379") };
    pub static ref REDIS_DATABASE: String = { default_env("REDIS_DATABASE", "") };
}
