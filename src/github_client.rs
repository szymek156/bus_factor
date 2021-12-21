use std::{error::Error};

use reqwest::header::USER_AGENT;

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
}

/// Sends a requests to given endpoint and returns a response body.
/// Returns ResponseError if query was invalid
pub async fn get_response_body(
    client: &GithubClient,
    endpoint: &str,
) -> Result<String, Box<dyn Error>> {
    const USER_AGENT_NAME: &str = "bus_factor";

    let res = client
        .inner
        .get(endpoint)
        .header(USER_AGENT, USER_AGENT_NAME)
        .bearer_auth(&client.token)
        .send()
        .await?;

    let is_error = res.error_for_status_ref().is_err();

    let body = res.text().await?;

    if is_error {
        // Api response contains useful information about the problem
        Err(Box::new(ResponseError::new(&body)))
    } else {
        Ok(body)
    }
}
