mod github_api;

use structopt::StructOpt;
use  github_api::get_repos;
#[derive(Debug, StructOpt)]
#[structopt(
    name = "bus_factor",
    about = "Command to gather bus factor statistics from gtihub repos.",

)]
struct Opt {
    /// Programming language name
    #[structopt(short, long)]
    language: String,

    /// Number of projects to consider
    #[structopt(short, long)]
    project_count: u32,
}

fn main() {
    // let opt = Opt::from_args();
    get_repos();
}
