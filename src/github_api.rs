use reqwest::header::USER_AGENT;
use reqwest::{self};
use serde::{Deserialize, Serialize};
use std::error::Error;
use std::io::Read;

use crate::api_errors::ResponseError;

//TODO: use enum where applicable
pub struct Query<'a> {
    pub language: &'a str,
    pub count: u32,
}
pub struct GithubApi {
    username: String,
    token: String,
    client: reqwest::blocking::Client,
}

struct BusFactor {
    bus_factor: f64,
    user_name: String,
}

#[derive(Serialize, Deserialize, Debug)]
/// Using serde for extracting data in interest from the github API response.
/// Response itself is enormous, this application uses only fraction of what
/// is exposed. It's achieved by defining a struct that has fields named as
/// in returned JSON. Awesomeness of serde library allows to pick up only
/// those elements we want, and put them to the struct.
/// It has many advantages.
/// - Visible only that data we want
/// - Open Close principle holds (wants something else? Simply add that field)
/// - Whole parsing and validation is done in one place:
/// ```
/// // If succeeds, we know all items are valid, can reach elements without fear
/// let contributions: Contributions = serde_json::from_str(&body)?;
/// let leader = contributions[0];
/// let biggest_contribution = leader.contributions;
///
/// ```
/// Instead of:
/// ```
/// // Check every single field, every single time
/// let biggest_contribution = leader["contributions"]
///    .as_u64()
///    .ok_or("Failed to retrieve contributions field")?;
///
/// let user_name = leader["login"]
///    .as_str()
///    .ok_or("Failed to retrieve login field")?;
/// ```
///
/// RepoData holds information about repository from the query
///
struct RepoData {
    contributors_url: String,
    name: String,
    stargazers_count: u64,
}
#[derive(Serialize, Deserialize, Debug)]
/// Repos holds list of items that are result from
/// https://api.github.com/search/repositories
struct Repos {
    items: Vec<RepoData>,
}

#[derive(Serialize, Deserialize, Debug)]
/// Keeps data about contributor
struct ContributorData {
    contributions: u64,
    login: String,
}

/// This is a list of items from
/// https://api.github.com/repos/USER/REPO/contributors
type Contributions = Vec<ContributorData>;

impl GithubApi {
    pub fn new(username: &str, token: &str) -> Self {
        Self {
            username: username.to_string(),
            token: token.to_string(),
            client: reqwest::blocking::Client::new(),
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

    /// Gets share of contribution for most active user among 25 others
    fn calculate_bus_factor(&self, contributors_url: &str) -> Result<BusFactor, Box<dyn Error>> {
        let endpoint = format!(
            "{contributors_url}?per_page={per_page}",
            contributors_url = contributors_url,
            per_page = 25
        );

        debug!("Got endpoint {}", endpoint);

        let body = self.get_response_body(&endpoint)?;

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

        let body = self.get_response_body(&endpoint)?;

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

    #[test]
    /// Checks if usage and value of the token are valid
    /// Test requires token to be in root/.token
    fn can_use_token() {
        let mut filepath = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        filepath.push(".token");

        let token = fs::read_to_string(filepath).expect("Something went wrong reading the file");

        let api = GithubApi::new("szymek156", &token);

        let endpoint = format!("https://api.github.com/users/{}/hovercard", api.username);

        let res = api
            .client
            .get(endpoint)
            .header(USER_AGENT, "bus_factor")
            .bearer_auth(&api.token)
            .send()
            .unwrap();

        assert_eq!(res.status(), StatusCode::OK);
    }
}
