use std::{fmt, error::Error};

#[derive(Debug)]
/// Error returned by API containing information from the server
pub struct ResponseError {
    details: String,
}

impl ResponseError {
    pub fn new(msg: &str) -> Self {
        Self {
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

#[derive(Debug)]
pub struct InvalidQueryError {
    details: String,

}

impl InvalidQueryError {
    pub fn new(msg: &str) -> Self {
        Self {
            details: msg.to_string(),
        }
    }
}

impl fmt::Display for InvalidQueryError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self.details)
    }
}

impl Error for InvalidQueryError {
    fn description(&self) -> &str {
        &self.details
    }
}