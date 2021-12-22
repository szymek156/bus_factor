use std::error::Error;

use async_trait::async_trait;
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

        let mut res = self
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

pub struct DefaultClientFactory;

trait ClientFactory {
    // fn create(token: &str) -> Box<dyn Testo>;
}

impl ClientFactory for DefaultClientFactory {
    // fn create(token: &str) -> Box<dyn Testo> {
    //     todo!()
    // }
}

#[cfg(test)]
mod tests {
    use std::{
        cell::RefCell,
        collections::{HashMap, VecDeque},
        fs,
        path::PathBuf,
        sync::Mutex,
    };

    use assert_approx_eq::assert_approx_eq;

    use reqwest::{header::USER_AGENT, StatusCode};
    use serde::{Deserialize, Serialize};

    use crate::{
        github_api::UserShare,
        github_data::{ContributorData, RepoData},
    };

    use super::*;

    /// Mock for the github client. Holds a collection of responses,
    /// that are consumed in FIFO order upon each call to
    /// get_response_body
    struct ClientMock {
        // Mutex is needed, to async closure be "Send"
        // RefCell for interior mutability, get_response_body
        // gets &self, but we need to be able to change the collection
        mock_response: Mutex<RefCell<VecDeque<String>>>,
    }

    impl ClientMock {
        fn new() -> Self {
            Self {
                mock_response: Default::default(),
            }
        }

        /// Put message on a queue, will be taken back by get_response_body
        /// Right now only Ok is supported
        fn shall_return<T>(&mut self, response: &T)
        where
            T: Serialize,
        {
            let s = serde_json::to_string(response).unwrap();

            self.mock_response.lock().unwrap().borrow_mut().push_back(s);
        }

        async fn get_response_body<T>(&self, _endpoint: &str) -> Result<T, Box<dyn Error>>
        where
            T: DeserializeOwned,
        {
            let guard = self.mock_response.lock().unwrap();

            let consumed = serde_json::from_str(&guard.borrow_mut().pop_front().unwrap()).unwrap();
            Ok(consumed)
        }
    }

    #[tokio::test]
    async fn simple_test() {
        #[derive(Debug, Serialize, Deserialize)]
        struct UserResponse {
            login: String,
            id: u32,
        }

        let mut filepath = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        filepath.push(".token");

        let token = fs::read_to_string(filepath).expect("Something went wrong reading the file");

        let client = GithubClient::new(&token);

        let res = client
            .get_response_body::<UserResponse>("https://api.github.com/user")
            .await
            .unwrap();

        println!("Got async resp {:?}", res);
    }

    #[tokio::test]
    async fn simple_mock_test() {
        // let mut client = ClientMock::new();

        // let sample = UserShare {
        //     bus_factor: 0.123,
        //     user_name: "user".to_string(),
        // };

        // client.shall_return(&sample);

        // let res = client
        //     .get_response_body::<UserShare>("endpoint")
        //     .await
        //     .unwrap();

        // println!("Got mock response {:?}", res);
    }
}
