use std::error::Error;
use std::fmt::Debug;

use crate::api_errors::InvalidQueryError;
use crate::github_client::GithubClient;
use crate::github_data::{Contributions, Repos};

// Max number of elements that fits on the page
const PAGE_LIMIT: u32 = 100;
const REPO_ENDPONT: &str = "https://api.github.com/search/repositories";
/// Contains parameters used for searching repositories
pub struct RepoQuery<'a> {
    pub language: &'a str,
    pub count: u32,
}

/// Parameters to characterize bus_factor calculation
pub struct BusFactorQuery {
    pub bus_threshold: f64,
    pub users_to_consider: u32,
}
/// Entity used to communicate with api.github.com
pub struct GithubApi {
    token: String,
}
#[derive(Debug, PartialEq)]
// Percentage user share in repository
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
            token: token.to_string(),
        }
    }

    /// For given count elements returns number of full pages, and residual
    fn get_pages(count: u32) -> (u32, u32) {
        // Number of pages with PAGE_LIMIT elements
        let full_pages = count / PAGE_LIMIT;
        // Last page that has the rest
        let last_page = count % PAGE_LIMIT;

        (full_pages, last_page)
    }

    /// Returns most popular projects (by stars) for given language in ascending order
    pub async fn get_repos(&self, query: &RepoQuery<'_>) -> Result<Repos, Box<dyn Error>> {
        let (full_pages, last_page) = GithubApi::get_pages(query.count);

        let query = format!(
            "?q=language:{language}&sort=stars&order=desc",
            language = query.language
        );

        let mut futures = vec![];
        // Accumulate repos from all full pages, page numbering starts from 1, not 0
        for page in 1..=full_pages {
            futures.push(self.get_repos_from_page(&query, page, PAGE_LIMIT));
        }

        // Execute all requests concurrently, responses are in the same order as futures
        let responses = futures::future::join_all(futures).await;

        let mut result = Repos::default();
        for res in responses {
            let repos = res?;
            result.items.extend_from_slice(&repos.items);
        }

        // Get number of pages for last request
        let last_page_elements = match full_pages {
            // If there are no full pages, get exactly last_page elements
            0 => last_page,
            // If there are full pages, get full page, to have pagination right
            _ => PAGE_LIMIT,
        };

        if last_page > 0 {
            let res = self
                .get_repos_from_page(&query, full_pages + 1, last_page_elements)
                .await?;
            // Get only last_page elements
            result
                .items
                .extend_from_slice(&res.items[0..last_page as usize]);
        }

        Ok(result)
    }

    /// Helper function that returns repositories on given page
    async fn get_repos_from_page(
        &self,
        query: &str,
        page: u32,
        per_page: u32,
    ) -> Result<Repos, Box<dyn Error>> {
        let endpoint = format!(
            "{endpoint}{query}&per_page={per_page}&page={page}",
            endpoint = REPO_ENDPONT,
            query = query,
            per_page = per_page,
            page = page
        );

        debug!("Repos endpoint {}", endpoint);

        // Create separate client for each call
        let repos = GithubClient::new(&self.token)
            .get_response_body::<Repos>(&endpoint)
            .await?;

        Ok(repos)
    }

    /// Calculates bus factor for each repo. Returns collection of repos that has
    /// factor significant.
    pub async fn get_repos_bus_factor(
        &self,
        repos: &Repos,
        query: &BusFactorQuery,
    ) -> Result<Vec<BusFactor>, Box<dyn Error>> {
        let mut futures = vec![];

        // Generate futures
        for item in &repos.items {
            futures
                .push(self.calculate_repo_share(&item.contributors_url, query.users_to_consider));
        }

        // Execute all requests concurrently, responses are in the same order as futures
        let responses = futures::future::join_all(futures).await;

        let mut res = Vec::<BusFactor>::new();

        // Well, unstable
        // for (response, repo) in zip(&responses, &repos.items)  {
        for (idx, item) in responses.into_iter().enumerate() {
            let share = item?;
            // responses, and repo has the same amount of elements
            let repo = &repos.items[idx];

            trace!(
                "Project {}, stars {} has bus factor {} for user {}",
                repo.name,
                repo.stargazers_count,
                share.bus_factor,
                share.user_name
            );

            if share.bus_factor >= query.bus_threshold {
                res.push(BusFactor {
                    repo_name: repo.name.to_owned(),
                    stars: repo.stargazers_count,
                    leader: share,
                })
            }
        }

        Ok(res)
    }

    /// Gets share of contribution for most active user among users_to_consider
    async fn calculate_repo_share(
        &self,
        contributors_url: &str,
        users_to_consider: u32,
    ) -> Result<UserShare, Box<dyn Error>> {
        if users_to_consider == 0 {
            // Such request does not make any sense
            return Err(Box::new(InvalidQueryError::new(
                "Number of users to consider must be greater than 0.",
            )));
        }

        let endpoint = format!(
            "{contributors_url}?per_page={per_page}",
            contributors_url = contributors_url,
            per_page = users_to_consider
        );

        trace!("Contributors endpoint {}", endpoint);

        let contributions = GithubClient::new(&self.token)
            .get_response_body::<Contributions>(&endpoint)
            .await?;

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

    // impl ClientMock {
    //     fn new() -> Self {
    //         Self {
    //             mock_response: RefCell::new(VecDeque::new()),
    //         }
    //     }

    //     fn shall_return(&mut self, response: Result<String, Box<dyn Error>>) {
    //         self.mock_response.borrow_mut().push_back(response);
    //     }
    // }

    // impl Client for ClientMock {
    //     fn get_response_body(&self, _endpoint: &str) -> Result<String, Box<dyn Error>> {
    //         let consumed = self.mock_response.borrow_mut().pop_front().unwrap();
    //         consumed
    //     }
    // }

    #[test]
    fn test_get_pages() {
        // Result that fits on one page
        let (full_pages, last_page) = GithubApi::get_pages(50);
        assert_eq!(full_pages, 0);
        assert_eq!(last_page, 50);

        // Result that comes to second page
        let (full_pages, last_page) = GithubApi::get_pages(101);
        assert_eq!(full_pages, 1);
        assert_eq!(last_page, 1);

        // Huuuge result
        let (full_pages, last_page) = GithubApi::get_pages(2531);
        assert_eq!(full_pages, 25);
        assert_eq!(last_page, 31);

        // Weird, but correct
        let (full_pages, last_page) = GithubApi::get_pages(0);
        assert_eq!(full_pages, 0);
        assert_eq!(last_page, 0);
    }

    #[tokio::test]
    /// Checks if usage and value of the token are valid
    /// Test requires token to be in root/.token
    async fn can_use_token() {
        let mut filepath = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        filepath.push(".token");

        let token = fs::read_to_string(filepath).expect("Something went wrong reading the file");
        let endpoint = "https://api.github.com/user";
        let client = reqwest::Client::new();

        let res = client
            .get(endpoint)
            .header(USER_AGENT, "bus_factor")
            .bearer_auth(&token)
            .send()
            .await
            .unwrap();

        assert_eq!(res.status(), StatusCode::OK);
    }

    // #[test]
    // /// Check that bus factor calculation is correct
    // fn get_repo_bus_factor_works() {
    //     let mut api = GithubApi::new(&"token");

    //     // Prepare some fake data
    //     let data = vec![
    //         ContributorData {
    //             contributions: 15,
    //             login: "user1".to_string(),
    //         },
    //         ContributorData {
    //             contributions: 3,
    //             login: "user2".to_string(),
    //         },
    //         ContributorData {
    //             contributions: 1,
    //             login: "user3".to_string(),
    //         },
    //     ];

    //     let data = serde_json::to_string(&data).unwrap();

    //     let mut mock = ClientMock::new();
    //     mock.shall_return(Ok(data));
    //     api.client = Box::new(mock);

    //     let repos = Repos {
    //         items: vec![RepoData::default()],
    //     };

    //     let res = api
    //         .get_repos_bus_factor(
    //             &repos,
    //             &BusFactorQuery {
    //                 bus_threshold: 0.75,
    //                 users_to_consider: 3,
    //             },
    //         )
    //         .unwrap();

    //     assert_eq!(res.len(), 1);

    //     assert_approx_eq!(res[0].leader.bus_factor, 0.789, 0.001);
    // }

    // #[test]
    // /// Simulate full flow
    // fn can_get_bus_factor_from_repos() {
    //     let mut api = GithubApi::new(&"token");
    //     let mut mock = ClientMock::new();

    //     // Prepare repos data
    //     let repos = serde_json::to_string(&Repos {
    //         items: vec![
    //             RepoData {
    //                 contributors_url: "https://api.github.com/repos/996icu/996.ICU/contributors"
    //                     .to_string(),
    //                 name: "996.ICU".to_string(),
    //                 stargazers_count: 260209,
    //             },
    //             RepoData {
    //                 contributors_url: "https://api.github.com/repos/denoland/deno/contributors"
    //                     .to_string(),
    //                 name: "deno".to_string(),
    //                 stargazers_count: 79306,
    //             },
    //             RepoData {
    //                 contributors_url: "https://api.github.com/repos/rust-lang/rust/contributors"
    //                     .to_string(),
    //                 name: "rust".to_string(),
    //                 stargazers_count: 61545,
    //             },
    //         ],
    //     })
    //     .unwrap();

    //     // Firstly return repos, when calling get_repos, some of
    //     // them has high bus_factor, but not all.
    //     mock.shall_return(Ok(repos));

    //     // Prepare contributions data
    //     // ... for repo 1
    //     let repo_contributions = serde_json::to_string(&vec![
    //         ContributorData {
    //             contributions: 1354,
    //             login: "996icu".to_string(),
    //         },
    //         ContributorData {
    //             contributions: 49,
    //             login: "ChangedenCZD".to_string(),
    //         },
    //         ContributorData {
    //             contributions: 26,
    //             login: "bofeiw".to_string(),
    //         },
    //     ])
    //     .unwrap();

    //     mock.shall_return(Ok(repo_contributions));

    //     // ... for repo 2
    //     let repo_contributions = serde_json::to_string(&vec![
    //         ContributorData {
    //             contributions: 1377,
    //             login: "ry".to_string(),
    //         },
    //         ContributorData {
    //             contributions: 838,
    //             login: "bartlomieju".to_string(),
    //         },
    //         ContributorData {
    //             contributions: 412,
    //             login: "piscisaureus".to_string(),
    //         },
    //     ])
    //     .unwrap();

    //     mock.shall_return(Ok(repo_contributions));

    //     // ... for repo 3
    //     let repo_contributions = serde_json::to_string(&vec![
    //         ContributorData {
    //             contributions: 22552,
    //             login: "bors".to_string(),
    //         },
    //         ContributorData {
    //             contributions: 5507,
    //             login: "brson".to_string(),
    //         },
    //         ContributorData {
    //             contributions: 5072,
    //             login: "alexcrichton".to_string(),
    //         },
    //     ])
    //     .unwrap();

    //     mock.shall_return(Ok(repo_contributions));

    //     api.client = Box::new(mock);

    //     // Simulate call to get_repos
    //     let repos = api
    //         .get_repos(&RepoQuery {
    //             language: &"rust",
    //             count: 3,
    //         })
    //         .unwrap();

    //     // Simulate call to get_repo_bus_factor
    //     let res = api
    //         .get_repos_bus_factor(
    //             &repos,
    //             &BusFactorQuery {
    //                 bus_threshold: 0.75,
    //                 users_to_consider: 3,
    //             },
    //         )
    //         .unwrap();

    //     // With given parameters only one repo should be returned
    //     let expected = vec![BusFactor {
    //         leader: UserShare {
    //             bus_factor: 0.947515745276417,
    //             user_name: "996icu".to_string(),
    //         },
    //         repo_name: "996.ICU".to_string(),
    //         stars: 260209,
    //     }];

    //     assert_eq!(expected, res);
    // }
}
