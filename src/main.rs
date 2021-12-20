#[macro_use]
extern crate log;

mod api_errors;
mod github_api;
mod github_client;
mod github_data;
use std::fs;

use github_api::{BusFactor, GithubApi, RepoQuery};
use structopt::StructOpt;

use crate::github_api::BusFactorQuery;
#[derive(Debug, StructOpt)]
#[structopt(
    name = "bus_factor",
    about = "Command to gather bus factor statistics from gtihub repos."
)]
struct Opt {
    /// Programming language name
    #[structopt(short, long)]
    language: String,

    /// Number of projects to consider
    #[structopt(short, long)]
    project_count: u32,

    /// Filepath for token
    #[structopt(short, long, default_value = "./.token")]
    token_path: String,
}

// TODO: better to pass str or String?
fn get_token(filepath: &str) -> String {
    let contents = fs::read_to_string(filepath).expect("Something went wrong reading the file");

    contents
}

fn show_result(res: &[BusFactor]) {
    for repo in res {
        println!(
            "project: {project:20} user: {user:20} percentage: {bus_factor:.2} stars: {stars}",
            project = repo.repo_name,
            user = repo.leader.user_name,
            bus_factor = repo.leader.bus_factor,
            stars = repo.stars
        )
    }
}

// TODO: async
// TODO: use anyhow, or something for err handling
// TODO: Add docs
// TODO: tests
// TODO: clippy
// TODO: read about bearer auth
// TODO: return errs with context
// TODO: test the cli
// TODO: 0 has a special meaning, returns all (30 - default per page)
// cargo run -- --language rust --project-count 0
fn main() {
    env_logger::init();

    let opt = Opt::from_args();

    let token = get_token(&opt.token_path);

    let api = GithubApi::new(&token);

    println!("Querying for repos...");
    let repos = api
        .get_repos(&RepoQuery {
            language: &opt.language,
            count: opt.project_count,
        })
        .unwrap();

    println!("Calculating bus factor for them...");
    let res = api
        .get_repos_bus_factor(
            &repos,
            &BusFactorQuery {
                bus_threshold: 0.75,
                users_to_consider: 25,
            },
        )
        .unwrap();

    show_result(&res);
}

#[cfg(test)]
/// Integration tests, use actual Github API
mod tests {
    use std::{collections::BTreeSet, fs, path::PathBuf};

    use crate::api_errors::ResponseError;

    use super::*;

    fn load_token() -> String {
        let mut filepath = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        filepath.push(".token");

        let token = fs::read_to_string(filepath).expect("Something went wrong reading the file");

        token
    }

    #[test]
    /// Simple call to the API
    fn simple_call_works() {
        let token = load_token();
        let api = GithubApi::new(&token);

        let res = api
            .get_repos(&RepoQuery {
                language: "rust",
                count: 1,
            })
            .unwrap();

        assert_eq!(res.items.len(), 1);
    }

    #[test]
    /// Request 0 elements, expect 0
    fn empty_call_does_not_blow_up() {
        let token = load_token();
        let api = GithubApi::new(&token);

        let repos = api
            .get_repos(&RepoQuery {
                language: "rust",
                count: 0,
            })
            .unwrap();

        assert_eq!(repos.items.len(), 0);

        let res = api.get_repos_bus_factor(
            &repos,
            &BusFactorQuery {
                bus_threshold: 0.75,
                users_to_consider: 25,
            },
        ).unwrap();

        assert_eq!(res.len(), 0);
    }

    #[test]
    // Requests # of repos that does not fit on one page
    fn pagination_works() {
        let token = load_token();
        let api = GithubApi::new(&token);

        let repo_count = 150;

        let repos = api
            .get_repos(&RepoQuery {
                language: "rust",
                count: repo_count,
            })
            .unwrap();

        // Expect to get that many repos as requested
        assert_eq!(repos.items.len(), repo_count as usize);

        // Expect no duplicates
        let set: BTreeSet<_> = repos.items.into_iter().collect();
        assert_eq!(set.len(), repo_count as usize);
    }

    #[test]
    /// Test failure on contributions endpoint
    fn api_fails_response_error_is_propagated() {
        let token = load_token();
        let api = GithubApi::new(&token);

        let repo = api
            .get_repos(&RepoQuery {
                language: "C",
                count: 1,
            })
            .unwrap();

        // Linux is C project, with too many contributions to show, api will fail
        let err = api
            .get_repos_bus_factor(
                &repo,
                &BusFactorQuery {
                    bus_threshold: 0.75,
                    users_to_consider: 25,
                },
            )
            .unwrap_err();

        assert!(err.is::<ResponseError>());
        // message, too many contributions to show via api
        // TODO: might want check the message too
    }

    #[test]
    /// Check failure on repos endpoint
    fn invalid_language() {
        let token = load_token();
        let api = GithubApi::new(&token);

        // Invalid language, api will fail
        let err = api
            .get_repos(&RepoQuery {
                language: "asdf",
                count: 1,
            })
            .unwrap_err();

        assert!(err.is::<ResponseError>());
        // message, invalid language
    }
}
