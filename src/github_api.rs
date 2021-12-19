use reqwest::header::USER_AGENT;
use reqwest::{self};
use std::error::Error;
use std::io::Read;

use crate::api_errors::ResponseError;
use crate::github_data::{Contributions, Repos};

struct GithubRequestor {
    client: reqwest::blocking::Client,
    token: String,
}

impl GithubRequestor {
    fn new(token: &str) -> Self {
        Self {
            client: reqwest::blocking::Client::new(),
            token: token.to_string(),
        }
    }
}
trait Requestor {
    fn get_response_body(&self, endpoint: &str) -> Result<String, Box<dyn Error>>;
}

impl Requestor for GithubRequestor {
    /// Sends a requests to given endpoint and returns a response body.
    /// Only when request was successful.
    fn get_response_body(&self, endpoint: &str) -> Result<String, Box<dyn Error>> {
        const USER_AGENT_NAME: &str = "bus_factor";

        let mut res = self
            .client
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
pub struct Query<'a> {
    pub language: &'a str,
    pub count: u32,
}
pub struct GithubApi {
    requestor: Box<dyn Requestor>,
}

struct BusFactor {
    bus_factor: f64,
    user_name: String,
}

impl GithubApi {
    pub fn new(token: &str) -> Self {
        Self {
            requestor: Box::new(GithubRequestor::new(token)),
        }
    }

    // TODO: implement
    /// For given count elements returns number of pages, and residual
    fn get_pages(count: u32) -> (u32, u32) {
        const PAGE_LIMIT: u32 = 100;
        if count > PAGE_LIMIT {
            // TODO: implement it
            todo!();
        }

        (0, count)

        // let pages = count / PAGE_LIMIT;
        // let last_page = count % PAGE_LIMIT;

        // (pages, last_page)
    }

    /// Gets share of contribution for most active user among 25 others
    fn calculate_bus_factor(&self, contributors_url: &str) -> Result<BusFactor, Box<dyn Error>> {
        let endpoint = format!(
            "{contributors_url}?per_page={per_page}",
            contributors_url = contributors_url,
            per_page = 25
        );

        debug!("Got endpoint {}", endpoint);

        let body = self.requestor.get_response_body(&endpoint)?;

        let contributions: Contributions = serde_json::from_str(&body)?;

        let total_contributions = contributions
            .iter()
            .fold(0, |acc, contr| acc + contr.contributions);

        // Assuming there is always at least one contribution
        // Contributions are sorted in descending order, so first element
        // is contributor with highest activity.
        let leader = &contributions[0];
        let bus_factor = leader.contributions as f64 / total_contributions as f64;

        Ok(BusFactor {
            user_name: leader.login.to_string(),
            bus_factor,
        })
    }

    /// Returns most popular projects (by stars) for given language in ascending order
    pub fn get_projects(&self, query: &Query) -> Result<(), Box<dyn Error>> {
        let (_pages, count) = GithubApi::get_pages(query.count);

        let endpoint =
        format!("https://api.github.com/search/repositories?q=language:{language}&sort=stars&order=desc&per_page={per_page}",
          language=query.language, per_page=count);

        let body = self.requestor.get_response_body(&endpoint)?;

        let repos: Repos = serde_json::from_str(&body)?;

        for item in &repos.items {
            let bus_factor = self.calculate_bus_factor(&item.contributors_url)?;

            if bus_factor.bus_factor >= 0.75 {
                println!(
                    "Project {}, stars {} has bus factor {} for user {}",
                    item.name, item.stargazers_count, bus_factor.bus_factor, bus_factor.user_name
                );
            }
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use std::{fs, path::PathBuf};

    use reqwest::StatusCode;

    use super::*;

    struct RequestorMock {
        mock_response: Result<String, Box<dyn Error>>,
    }

    impl RequestorMock {
        fn new() -> Self {
            Self {
                mock_response: Ok("{}".to_string()),
            }
        }

        fn shall_return(&mut self, response: Result<String, Box<dyn Error>>) {
            self.mock_response = response;
        }
    }

    impl Requestor for RequestorMock {
        fn get_response_body(&self, endpoint: &str) -> Result<String, Box<dyn Error>> {
            Ok("{}".to_string())
        }
    }

    fn load_token() -> String {
        let mut filepath = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        filepath.push(".token");

        let token = fs::read_to_string(filepath).expect("Something went wrong reading the file");

        token
    }

    #[test]
    /// Checks if usage and value of the token are valid
    /// Test requires token to be in root/.token
    fn can_use_token() {
        let token = load_token();
        let client = reqwest::blocking::Client::new();

        let endpoint = format!("https://api.github.com/user");

        let res = client
            .get(endpoint)
            .header(USER_AGENT, "bus_factor")
            .bearer_auth(&token)
            .send()
            .unwrap();

        assert_eq!(res.status(), StatusCode::OK);
    }

    #[test]
    fn test_inject() {
        let token = load_token();

        let mut api = GithubApi::new(&token);

        let mut mock = RequestorMock::new();
        mock.shall_return(Ok("{}".to_string()));
        api.requestor = Box::new(mock);

        let res = api.get_projects(&Query {
            language: "rust",
            count: 10,
        });

        assert!(res.is_ok());
    }
}
