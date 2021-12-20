use std::error::Error;
use std::fmt::Debug;

use crate::github_client::{Client, GithubClient};
use crate::github_data::{Contributions, Repos};

/// Contains parameters used for searching repositories
pub struct RepoQuery<'a> {
    pub language: &'a str,
    pub count: u32,
}

pub struct BusFactorQuery {
    pub bus_threshold: f64,
    pub users_to_consider: u32,
}
/// Entity used to communicate with api.github.com
pub struct GithubApi {
    client: Box<dyn Client>,
}

#[derive(Debug, PartialEq)]
pub struct UserShare {
    pub bus_factor: f64,
    pub user_name: String,
}

#[derive(Debug, PartialEq)]
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
    fn calculate_repo_share(
        &self,
        contributors_url: &str,
        users_to_consider: u32,
    ) -> Result<UserShare, Box<dyn Error>> {
        let endpoint = format!(
            "{contributors_url}?per_page={per_page}",
            contributors_url = contributors_url,
            per_page = users_to_consider
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
    pub fn get_repo_bus_factor(
        &self,
        repos: &Repos,
        query: &BusFactorQuery,
    ) -> Result<Vec<BusFactor>, Box<dyn Error>> {
        let mut res = Vec::<BusFactor>::new();

        for item in &repos.items {
            let share =
                self.calculate_repo_share(&item.contributors_url, query.users_to_consider)?;
            debug!(
                "Project {}, stars {} has bus factor {} for user {}",
                item.name, item.stargazers_count, share.bus_factor, share.user_name
            );

            if share.bus_factor >= query.bus_threshold {
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
    use std::{cell::RefCell, collections::VecDeque, fs, path::PathBuf};

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

    #[test]
    /// Checks if usage and value of the token are valid
    /// Test requires token to be in root/.token
    fn can_use_token() {
        let mut filepath = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        filepath.push(".token");

        let token = fs::read_to_string(filepath).expect("Something went wrong reading the file");

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
    /// Check that bus factor calculation is correct
    fn get_repo_bus_factor_works() {
        let mut api = GithubApi::new(&"token");

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

        let res = api
            .get_repo_bus_factor(
                &repos,
                &BusFactorQuery {
                    bus_threshold: 0.75,
                    users_to_consider: 3,
                },
            )
            .unwrap();

        assert_eq!(res.len(), 1);

        assert_approx_eq!(res[0].leader.bus_factor, 0.789, 0.001);
    }

    #[test]
    /// Simulate full flow
    fn can_get_bus_factor_from_repos() {
        let mut api = GithubApi::new(&"token");
        let mut mock = ClientMock::new();

        // Prepare repos data
        let repos = serde_json::to_string(&Repos {
            items: vec![
                RepoData {
                    contributors_url: "https://api.github.com/repos/996icu/996.ICU/contributors"
                        .to_string(),
                    name: "996.ICU".to_string(),
                    stargazers_count: 260209,
                },
                RepoData {
                    contributors_url: "https://api.github.com/repos/denoland/deno/contributors"
                        .to_string(),
                    name: "deno".to_string(),
                    stargazers_count: 79306,
                },
                RepoData {
                    contributors_url: "https://api.github.com/repos/rust-lang/rust/contributors"
                        .to_string(),
                    name: "rust".to_string(),
                    stargazers_count: 61545,
                },
            ],
        })
        .unwrap();

        // Firstly return repos, when calling get_repos, some of
        // them has high bus_factor, but not all.
        mock.shall_return(Ok(repos));

        // Prepare contributions data
        // ... for repo 1
        let repo_contributions = serde_json::to_string(&vec![
            ContributorData {
                contributions: 1354,
                login: "996icu".to_string(),
            },
            ContributorData {
                contributions: 49,
                login: "ChangedenCZD".to_string(),
            },
            ContributorData {
                contributions: 26,
                login: "bofeiw".to_string(),
            },
        ])
        .unwrap();

        mock.shall_return(Ok(repo_contributions));

        // ... for repo 2
        let repo_contributions = serde_json::to_string(&vec![
            ContributorData {
                contributions: 1377,
                login: "ry".to_string(),
            },
            ContributorData {
                contributions: 838,
                login: "bartlomieju".to_string(),
            },
            ContributorData {
                contributions: 412,
                login: "piscisaureus".to_string(),
            },
        ])
        .unwrap();

        mock.shall_return(Ok(repo_contributions));

        // ... for repo 3
        let repo_contributions = serde_json::to_string(&vec![
            ContributorData {
                contributions: 22552,
                login: "bors".to_string(),
            },
            ContributorData {
                contributions: 5507,
                login: "brson".to_string(),
            },
            ContributorData {
                contributions: 5072,
                login: "alexcrichton".to_string(),
            },
        ])
        .unwrap();

        mock.shall_return(Ok(repo_contributions));

        api.client = Box::new(mock);

        // Simulate call to get_repos
        let repos = api
            .get_repos(&RepoQuery {
                language: &"rust",
                count: 3,
            })
            .unwrap();

        // Simulate call to get_repop_bus_factor
        let res = api
            .get_repo_bus_factor(
                &repos,
                &BusFactorQuery {
                    bus_threshold: 0.75,
                    users_to_consider: 3,
                },
            )
            .unwrap();

        // With given parameters only one repo should be returned
        let expected = vec![BusFactor {
            leader: UserShare {
                bus_factor: 0.947515745276417,
                user_name: "996icu".to_string(),
            },
            repo_name: "996.ICU".to_string(),
            stars: 260209,
        }];

        assert_eq!(expected, res);
    }
}
