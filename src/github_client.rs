use std::{error::Error, io::Read};

use reqwest::header::USER_AGENT;

use crate::api_errors::ResponseError;

// TODO: docs

pub struct GithubClient {
    inner: reqwest::blocking::Client,
    token: String,
}

impl GithubClient {
    pub fn new(token: &str) -> Self {
        Self {
            inner: reqwest::blocking::Client::new(),
            token: token.to_string(),
        }
    }
}

pub trait Client {
    fn get_response_body(&self, endpoint: &str) -> Result<String, Box<dyn Error>>;
}

impl Client for GithubClient {
    /// Sends a requests to given endpoint and returns a response body.
    /// Returns ResponseError if query was invalid
    fn get_response_body(&self, endpoint: &str) -> Result<String, Box<dyn Error>> {
        const USER_AGENT_NAME: &str = "bus_factor";

        let mut res = self
            .inner
            .get(endpoint)
            .header(USER_AGENT, USER_AGENT_NAME)
            .bearer_auth(&self.token)
            .send()?;

        let mut body = String::new();

        res.read_to_string(&mut body)?;

        // If status code is 4xx, 5xx
        if res.error_for_status().is_err() {
            // Api response contains useful information about the problem
            Err(Box::new(ResponseError::new(&body)))
        } else {
            Ok(body)
        }
    }
}
