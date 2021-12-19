#[macro_use]
extern crate log;

mod api_errors;
mod github_api;
mod github_client;
mod github_data;
use std::fs;

use github_api::{BusFactor, GithubApi, RepoQuery};
use structopt::StructOpt;
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
            "project: {project:20} user: {user:20} percentage: {bus_factor:.2}",
            project = repo.repo_name,
            user = repo.leader.user_name,
            bus_factor = repo.leader.bus_factor
        )
    }
}

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
    let res = api.get_repo_bus_factor(&repos).unwrap();

    show_result(&res);
}

#[cfg(test)]
/// Integration tests, use actual Github API
mod tests {
    use std::{error::Error, fs, path::PathBuf};

    use crate::api_errors::ResponseError;

    use super::*;

    fn load_token() -> String {
        let mut filepath = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        filepath.push(".token");

        let token = fs::read_to_string(filepath).expect("Something went wrong reading the file");

        token
    }

    #[test]
    /// Checks if usage and value of the token are valid
    /// Test requires token to be in root/.token
    fn simple_call_works() {
        let token = load_token();
        let api = GithubApi::new(&token);

        api.get_repos(&RepoQuery {
            language: "rust",
            count: 1,
        })
        .unwrap();
    }

    #[test]
    fn api_fails_message_is_propagated() {
        let token = load_token();
        let api = GithubApi::new(&token);

        let repo = api
            .get_repos(&RepoQuery {
                language: "C",
                count: 1,
            })
            .unwrap();

        let e = format!(
            "{:?}",
            Box::new(ResponseError::new(
                r#"{"message":"The history or contributor list is too large to list contributors for this repository via the API.","documentation_url":"https://docs.github.com/rest/reference/repos#list-repository-contributors"}"#
            ))
        );

        let r = format!("{:?}", api.get_repo_bus_factor(&repo).unwrap_err());
        // Linux is C project, with too many contributions to show, api will fail
        assert_eq!(e, r);
    }

    #[test]
    fn invalid_language() {
        let token = load_token();
        let api = GithubApi::new(&token);

        // TODO: try to figure better way
        let e = format!(
            "{:?}",
            Box::new(ResponseError::new(
                r#"{"message":"Validation Failed","errors":[{"message":"None of the search qualifiers apply to this search type.","resource":"Search","field":"q","code":"invalid"}],"documentation_url":"https://docs.github.com/v3/search/"}"#
            ))
        );

        let r = format!(
            "{:?}",
            api.get_repos(&RepoQuery {
                language: "asdf",
                count: 1,
            })
            .unwrap_err()
        );
        // Linux is C project, with too many contributions to show, api will fail
        assert_eq!(e, r);
    }
}
