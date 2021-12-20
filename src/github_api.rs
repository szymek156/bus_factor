use std::error::Error;
use std::fmt::Debug;

use crate::github_client::{Client, GithubClient};
use crate::github_data::{Contributions, Repos};

/// Contains parameters used for searching repositories
pub struct RepoQuery<'a> {
    pub language: &'a str,
    pub count: u32,
}
/// Entity used to communicate with api.github.com
pub struct GithubApi {
    client: Box<dyn Client>,
}

#[derive(Debug)]
pub struct UserShare {
    pub bus_factor: f64,
    pub user_name: String,
}

#[derive(Debug)]
/// Contains repo information together with most active user
pub struct BusFactor {
    pub leader: UserShare,
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
    fn calculate_repo_share(&self, contributors_url: &str) -> Result<UserShare, Box<dyn Error>> {
        let endpoint = format!(
            "{contributors_url}?per_page={per_page}",
            contributors_url = contributors_url,
            per_page = 25
        );

        debug!("Contributors endpoint {}", endpoint);

        let body = self.client.get_response_body(&endpoint)?;

        info!("Contributors data \n {}", body);

        let contributions: Contributions = serde_json::from_str(&body)?;

        let total_contributions = contributions
            .iter()
            .fold(0, |acc, contr| acc + contr.contributions);

        // Assuming there is always at least one contribution
        // Contributions are sorted in descending order, so first element
        // is contributor with highest activity.
        let leader = &contributions[0];
        let bus_factor = leader.contributions as f64 / total_contributions as f64;

        Ok(UserShare {
            user_name: leader.login.to_string(),
            bus_factor,
        })
    }

    /// Returns most popular projects (by stars) for given language in ascending order
    pub fn get_repos(&self, query: &RepoQuery) -> Result<Repos, Box<dyn Error>> {
        let (_pages, count) = GithubApi::get_pages(query.count);

        let endpoint =
        format!("https://api.github.com/search/repositories?q=language:{language}&sort=stars&order=desc&per_page={per_page}",
          language=query.language, per_page=count);

        debug!("Repos endpoint {}", endpoint);

        let body = self.client.get_response_body(&endpoint)?;

        let repos: Repos = serde_json::from_str(&body)?;

        Ok(repos)
    }

    /// Calculates bus factor for each repo. Returns collection of repos that has
    /// factor significant.
    pub fn get_repo_bus_factor(&self, repos: &Repos) -> Result<Vec<BusFactor>, Box<dyn Error>> {
        let mut res = Vec::<BusFactor>::new();

        for item in &repos.items {
            let share = self.calculate_repo_share(&item.contributors_url)?;
            debug!(
                "Project {}, stars {} has bus factor {} for user {}",
                item.name, item.stargazers_count, share.bus_factor, share.user_name
            );

            // TODO: parametrize
            if share.bus_factor >= 0.75 {
                res.push(BusFactor {
                    repo_name: item.name.to_owned(),
                    stars: item.stargazers_count,
                    leader: share,
                })
            }
        }

        Ok(res)
    }
}

#[cfg(test)]
mod tests {
    use std::{
        cell::RefCell,
        collections::{vec_deque, VecDeque},
        fs, mem,
        path::PathBuf,
    };

    use assert_approx_eq::assert_approx_eq;

    use reqwest::{header::USER_AGENT, StatusCode};

    use crate::github_data::{ContributorData, RepoData};

    use super::*;

    /// Mock for the github client. Holds a collection of responses,
    /// that are consumed in FIFO order upon each call to
    /// get_response_body
    struct ClientMock {
        mock_response: RefCell<VecDeque<Result<String, Box<dyn Error>>>>,
    }

    impl ClientMock {
        fn new() -> Self {
            Self {
                mock_response: RefCell::new(VecDeque::new()),
            }
        }

        fn shall_return(&mut self, response: Result<String, Box<dyn Error>>) {
            self.mock_response.borrow_mut().push_back(response);
        }
    }

    impl Client for ClientMock {
        fn get_response_body(&self, _endpoint: &str) -> Result<String, Box<dyn Error>> {
            let consumed = self.mock_response.borrow_mut().pop_front().unwrap();
            consumed
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
    fn get_repo_bus_factor_works() {
        let token = load_token();
        let mut api = GithubApi::new(&token);

        // Prepare some fake data
        let data = vec![
            ContributorData {
                contributions: 15,
                login: "user1".to_string(),
            },
            ContributorData {
                contributions: 3,
                login: "user2".to_string(),
            },
            ContributorData {
                contributions: 1,
                login: "user3".to_string(),
            },
        ];

        let data = serde_json::to_string(&data).unwrap();

        let mut mock = ClientMock::new();
        mock.shall_return(Ok(data));
        api.client = Box::new(mock);

        let repos = Repos {
            items: vec![RepoData::default()],
        };

        let res = api.get_repo_bus_factor(&repos).unwrap();

        assert_eq!(res.len(), 1);

        assert_approx_eq!(res[0].leader.bus_factor, 0.789, 0.001);
    }
}
