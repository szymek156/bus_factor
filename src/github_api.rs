use std::error::Error;
use std::fmt::Debug;

use crate::github_client::{Client, GithubClient};
use crate::github_data::{Contributions, Repos};

pub struct Query<'a> {
    pub language: &'a str,
    pub count: u32,
}
pub struct GithubApi {
    client: Box<dyn Client>,
}

#[derive(Debug)]
pub struct BusFactor {
    pub bus_factor: f64,
    pub user_name: String,
}

#[derive(Debug)]
pub struct ohgod {
    pub leader: BusFactor,
    pub repo_name: String,
    pub stars: u64,
}

impl GithubApi {
    pub fn new(token: &str) -> Self {
        Self {
            client: Box::new(GithubClient::new(token)),
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

        debug!("Contributors endpoint {}", endpoint);

        let body = self.client.get_response_body(&endpoint)?;

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
    pub fn get_projects(&self, query: &Query) -> Result<Vec<ohgod>, Box<dyn Error>> {
        let (_pages, count) = GithubApi::get_pages(query.count);

        let endpoint =
        format!("https://api.github.com/search/repositories?q=language:{language}&sort=stars&order=desc&per_page={per_page}",
          language=query.language, per_page=count);

        debug!("Repos endpoint {}", endpoint);

        let body = self.client.get_response_body(&endpoint)?;

        let repos: Repos = serde_json::from_str(&body)?;

        let mut res = Vec::<ohgod>::new();

        for item in &repos.items {
            let bus_factor = self.calculate_bus_factor(&item.contributors_url)?;
            debug!(
                "Project {}, stars {} has bus factor {} for user {}",
                item.name, item.stargazers_count, bus_factor.bus_factor, bus_factor.user_name
            );

            if bus_factor.bus_factor >= 0.75 {
                res.push(ohgod {
                    repo_name: item.name.to_owned(),
                    stars: item.stargazers_count,
                    leader: bus_factor,
                })
            }
        }

        Ok(res)
    }
}

#[cfg(test)]
mod tests {
    use std::{fs, path::PathBuf};

    use reqwest::{header::USER_AGENT, StatusCode};

    use super::*;

    struct ClientMock {
        mock_response: Result<String, Box<dyn Error>>,
    }

    impl ClientMock {
        fn new() -> Self {
            Self {
                mock_response: Ok("{}".to_string()),
            }
        }

        fn shall_return(&mut self, response: Result<String, Box<dyn Error>>) {
            self.mock_response = response;
        }
    }

    impl Client for ClientMock {
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

        let mut mock = ClientMock::new();
        mock.shall_return(Ok("{}".to_string()));
        api.client = Box::new(mock);

        let res = api.get_projects(&Query {
            language: "rust",
            count: 10,
        });

        assert!(res.is_ok());
    }
}
