use std::collections::VecDeque;
use std::error::Error;
use std::fmt::Debug;
use std::time::Duration;

use futures::Future;

use crate::api_errors::InvalidQueryError;
use crate::github_client::GithubClient;
use crate::github_data::{Contributions, Repos, RepoData};

// Max number of elements that fits on the page
const PAGE_LIMIT: u32 = 100;
const REPO_ENDPONT: &str = "https://api.github.com/search/repositories";
/// Contains parameters used for searching repositories
#[derive(Debug)]
pub struct RepoQuery<'a> {
    pub language: &'a str,
    pub count: u32,
    pub delay_sec: f64,
}

/// Parameters to characterize bus_factor calculation
pub struct BusFactorQuery {
    pub bus_threshold: f64,
    pub users_to_consider: u32,
    pub delay_sec: f64,

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

    /// Returns most popular projects (by stars) for given language in descending order
    pub async fn get_repos(&self, repo_query: &RepoQuery<'_>) -> Result<Repos, Box<dyn Error>> {
        let (full_pages, last_page) = GithubApi::get_pages(repo_query.count);

        let query = format!(
            "?q=language:{language}&sort=stars&order=desc",
            language = repo_query.language
        );

        let mut futures = vec![];
        // Accumulate repos from all full pages, page numbering starts from 1, not 0
        for page in 1..=full_pages {
            futures.push(self.get_repos_from_page(&query, page, PAGE_LIMIT));
        }

        if last_page > 0 {
            // Get number of pages for last request
            let last_page_elements = match full_pages {
                // If there are no full pages, get exactly last_page elements
                0 => last_page,
                // If there are full pages, get full page, to have pagination right
                _ => PAGE_LIMIT,
            };
            futures.push(self.get_repos_from_page(&query, full_pages + 1, last_page_elements));
        }

        let mut responses = Vec::new();
        dbg!(repo_query);
        if repo_query.delay_sec > 0.0 {
            for element in futures {
                responses.push(element.await);
                println!("after push");
                tokio::time::sleep(Duration::from_secs_f64(repo_query.delay_sec)).await;
                println!("after sleep");

            }

        } else {
            // Execute all requests concurrently, responses are in the same order as futures
            responses = futures::future::join_all(futures).await;
        }

        // Last element is treated differently, exclude it from following iteration
        let mut last_response = None;
        if last_page > 0 {
            last_response = responses.pop();
        }

        let mut result = Repos::default();
        for res in responses {
            let repos = res?;
            result.items.extend_from_slice(&repos.items);
        }

        if let Some(last_result) = last_response {
            let last_repos = last_result?;
            // From last result get exactly last_page elements
            result
                .items
                .extend_from_slice(&last_repos.items[0..last_page as usize])
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

    async fn worker(&self, jobs : &[RepoData], query: &BusFactorQuery) -> Vec<Result<UserShare, Box<dyn Error>>> {
        let mut local_res = Vec::new();

        for job in jobs {
            info!("getting repo share...");
            let res = self.calculate_repo_share(&job.contributors_url, query.users_to_consider).await;

            local_res.push(res);

            tokio::time::sleep(Duration::from_secs_f64(query.delay_sec)).await;
        }

        local_res
    }

    /// Calculates bus factor for each repo. Returns collection of repos that has
    /// factor significant.
    pub async fn get_repos_bus_factor(
        &self,
        repos: &Repos,
        query: &BusFactorQuery,
    ) -> Result<Vec<BusFactor>, Box<dyn Error>> {
        let mut futures = Vec::new();

        // Generate futures
        // for item in &repos.items {
        //     futures
        //         .push(self.calculate_repo_share(&item.contributors_url, query.users_to_consider));
        // }

        // Number of api calls that can be executed at once
        let n_workers = 5;

        let jobs_count = repos.items.len() / n_workers;

        // Last chunk may contain less than jobs_count elements, the remainder
        for chunk in repos.items.chunks(jobs_count) {
            futures.push(self.worker(chunk, query));
        }


        let responses = futures::future::join_all(futures).await;

        // flatten partial results to vector of all results
        let responses : Vec<Result<UserShare, Box<dyn Error>>> = responses.into_iter().flatten().collect();
        // if query.delay_sec > 0.0 {
        //     for (idx, element) in futures.into_iter().enumerate() {
        //         let repo = &repos.items[idx];

        //         let share = element.await;
        //         tokio::time::sleep(Duration::from_secs_f64(query.delay_sec)).await;

        //         if let Ok(s) = &share {

        //             info!(
        //                 "Project {}, stars {} has bus factor {} for user {}",
        //                 repo.name,
        //                 repo.stargazers_count,
        //                 s.bus_factor,
        //                 s.user_name
        //             );
        //         }

        //         responses.push(share);
        //     }
        // } else {
            // // Execute all requests concurrently, responses are in the same order as futures
            // for chunk in futures.chunks(10) {

            //     responses.append(futures::future::join_all(chunk).await);
            // }
        // }


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
    use std::{fs, path::PathBuf};

    use reqwest::{header::USER_AGENT, StatusCode};

    use super::*;

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
}
