use std::{fmt, error::Error};

#[derive(Debug)]
/// Error returned by API containing information from the server
pub struct ResponseError {
    details: String,
}

impl ResponseError {
    pub fn new(msg: &str) -> ResponseError {
        ResponseError {
            details: msg.to_string(),
        }
    }
}

impl fmt::Display for ResponseError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self.details)
    }
}

impl Error for ResponseError {
    fn description(&self) -> &str {
        &self.details
    }
}
