use iron::error::Error;
use iron::prelude::*;
use iron::status;
use std::fmt;

struct StatusError {
    status: String,
}

impl Error for StatusError {
    fn description(&self) -> &str {
        return &*self.status;
    }
}

impl fmt::Display for StatusError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.status)
    }
}

impl fmt::Debug for StatusError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.status)
    }
}

pub fn status_error<T>(http_status: status::Status) -> Result<T, IronError> {
    return Err(IronError::new(
        Box::new(StatusError {
            status: format!("{}", http_status),
        }),
        http_status,
    ));
}
