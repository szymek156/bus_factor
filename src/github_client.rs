use std::error::Error;

use reqwest::header::USER_AGENT;
use serde::de::DeserializeOwned;

use crate::api_errors::ResponseError;
/// Entity that takes care on transport layer
pub struct GithubClient {
    inner: reqwest::Client,
    token: String,
}

impl GithubClient {
    pub fn new(token: &str) -> Self {
        Self {
            inner: reqwest::Client::new(),
            token: token.to_string(),
        }
    }
    /// Sends a requests to given endpoint and returns a response body.
    /// Returns ResponseError if query was invalid
    pub async fn get_response_body<T>(&self, endpoint: &str) -> Result<T, Box<dyn Error>>
    where
        T: DeserializeOwned,
    {
        const USER_AGENT_NAME: &str = "bus_factor";

        let res = self
            .inner
            .get(endpoint)
            .header(USER_AGENT, USER_AGENT_NAME)
            .bearer_auth(&self.token)
            .send()
            .await?;

        // If status code is 4xx, 5xx
        if res.error_for_status_ref().is_err() {
            // Api response contains useful information about the problem
            let body = res.text().await?;
            Err(Box::new(ResponseError::new(&body)))
        } else {
            let body: T = res.json().await?;
            Ok(body)
        }
    }
}

