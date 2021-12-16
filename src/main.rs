mod github_api;

use std::fs;

use github_api::GithubApi;
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
    // TODO: why there is a newline?
    let mut contents = fs::read_to_string(filepath).expect("Something went wrong reading the file");

    contents
}

fn main() {
    let opt = Opt::from_args();

    let token = get_token(&opt.token_path);

    let api = GithubApi::new(&opt.username, &token);

    api.requires_token();
}
