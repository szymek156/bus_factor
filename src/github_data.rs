//! Using serde for extracting data in interest from the github API response.
//! Response itself is enormous, this application uses only fraction of what
//! is exposed. It's achieved by defining a struct that has fields named as
//! in returned JSON. Awesomeness of serde library allows to pick up only
//! those elements we want, and put them to the struct.
//! It has many advantages.
//! - Visible only that data we want
//! - Open Close principle holds (wants something else? Simply add that field)
//! - Whole parsing and validation is done in one place:
//! ```
//! // If succeeds, we know all items are valid, can reach elements without fear
//! let contributions: Contributions = serde_json::from_str(&body)?;
//! let leader = contributions[0];
//! let biggest_contribution = leader.contributions;
//!
//! ```
//! Instead of:
//! ```
//! // Check every single field, every single time
//! let biggest_contribution = leader["contributions"]
//!    .as_u64()
//!    .ok_or("Failed to retrieve contributions field")?;
//!
//! let user_name = leader["login"]
//!    .as_str()
//!    .ok_or("Failed to retrieve login field")?;
//! ```
//!
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug)]

/// RepoData holds information about repository from the query
pub struct RepoData {
    pub contributors_url: String,
    pub name: String,
    pub stargazers_count: u64,
}
#[derive(Serialize, Deserialize, Debug)]
/// Repos holds list of items that are result from
/// https://api.github.com/search/repositories
pub struct Repos {
    pub items: Vec<RepoData>,
}

#[derive(Serialize, Deserialize, Debug)]
/// Keeps data about contributor
pub struct ContributorData {
    pub contributions: u64,
    pub login: String,
}

/// This is a list of items from
/// https://api.github.com/repos/USER/REPO/contributors
pub type Contributions = Vec<ContributorData>;
