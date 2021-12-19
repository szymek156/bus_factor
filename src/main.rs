#[macro_use]
extern crate log;

mod api_errors;
mod github_api;
mod github_data;
use std::fs;

use github_api::{GithubApi, Query};
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

    // TODO: username is needed?
    #[structopt(short, long, default_value = "szymek156")]
    username: String,
}

// TODO: better to pass str or String?
pub fn get_token(filepath: &str) -> String {
    let contents = fs::read_to_string(filepath).expect("Something went wrong reading the file");

    contents
}
// TODO: use anyhow, or something for err handling
// TODO: Add docs
// TODO: tests
// TODO: add a test for token
// TODO: clippy
// TODO: pretty formatting of the result
// TODO: read about bearer auth
// TODO: get rid of username
// TODO: return errs with context
// TODO: test the cli
// TODO: 0 has a special meaning, returns all (30 - default per page)
// cargo run -- --language rust --project-count 0
fn main() {
    env_logger::init();

    let opt = Opt::from_args();

    let token = get_token(&opt.token_path);

    let api = GithubApi::new(&opt.username, &token);

    api.get_projects(&Query {
        language: &opt.language,
        count: opt.project_count,
    })
    .unwrap();
}

#[cfg(test)]
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
        let api = GithubApi::new("szymek156", &token);

        api.get_projects(&Query {
            language: "rust",
            count: 1,
        })
        .unwrap();
    }

    #[test]
    fn api_fails_message_is_propagated() {
        let token = load_token();
        let api = GithubApi::new("szymek156", &token);

        let e = format!(
            "{:?}",
            Box::new(ResponseError::new(
                r#"{"message":"The history or contributor list is too large to list contributors for this repository via the API.","documentation_url":"https://docs.github.com/rest/reference/repos#list-repository-contributors"}"#
            ))
        );

        let r = format!(
            "{:?}",
            api.get_projects(&Query {
                language: "C",
                count: 1,
            })
            .unwrap_err()
        );
        // Linux is C project, with too many contributions to show, api will fail
        assert_eq!(e, r);
    }

    #[test]
    fn invalid_language() {
        let token = load_token();
        let api = GithubApi::new("szymek156", &token);

        let e = format!(
            "{:?}",
            Box::new(ResponseError::new(
                r#"{"message":"Validation Failed","errors":[{"message":"None of the search qualifiers apply to this search type.","resource":"Search","field":"q","code":"invalid"}],"documentation_url":"https://docs.github.com/v3/search/"}"#
            ))
        );

        let r = format!(
            "{:?}",
            api.get_projects(&Query {
                language: "asdf",
                count: 1,
            })
            .unwrap_err()
        );
        // Linux is C project, with too many contributions to show, api will fail
        assert_eq!(e, r);
    }
}
