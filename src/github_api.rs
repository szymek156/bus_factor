use reqwest;
use reqwest::header::USER_AGENT;
use serde_json::Value;
use std::error::Error;
use std::fs;
use std::io::Read;

//TODO: use enum where applicable
pub struct Query<'a> {
    pub language: &'a str,
    pub count: u32,
}
pub struct GithubApi {
    username: String,
    token: String,
}

struct BusFactor {
    bus_factor: f64,
    user_name: String,
}

impl GithubApi {
    pub fn new(username: &str, token: &str) -> Self {
        Self {
            username: username.to_string(),
            token: token.to_string(),
        }
    }

    // TODO: do a test
    /// Test token, this endpoint requires basic auth
    pub fn requires_token(&self) -> Result<(), reqwest::Error> {
        let client = reqwest::blocking::Client::new();

        let endpoint = format!("https://api.github.com/users/{}/hovercard", self.username);

        let mut res = client
            // .get("https://api.github.com/search/repositories?q=tetris+language:assembly&sort=stars&order=desc&per_page=5")
            .get(endpoint)
            .header(USER_AGENT, "bus_factor")
            .basic_auth(&self.username, Some(&self.token))
            .send()?;

        let mut body = String::new();

        res.read_to_string(&mut body);

        println!("Status: {}", res.status());
        println!("Body:\n{}", body);

        Ok(())
    }

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
    fn calculate_bus_factor(&self, contributors_url: &str) -> Result<BusFactor, Box<dyn Error>> {
        let client = reqwest::blocking::Client::new();

        let endpoint = format!(
            "{contributors_url}?per_page={per_page}",
            contributors_url = contributors_url,
            per_page = 25
        );

        let mut res = client
            .get(endpoint)
            .header(USER_AGENT, "bus_factor")
            .basic_auth(&self.username, Some(&self.token))
            .send()?;

        let mut body = String::new();

        res.read_to_string(&mut body)?;

        // println!("Status: {}", res.status());

        // TODO: reqwest error, and serde error, how to propagate both?
        let body: Value = serde_json::from_str(&body)?;

        let total_contributions = body
            .as_array()
            .ok_or("Failed to get an array of contributors")?
            .iter()
            .try_fold(0, |acc, contributor| {
                match contributor["contributions"].as_u64() {
                    Some(c) => Ok(acc + c),
                    None => Err("failed to get contribution"),
                }
            })?;

        let leader = &body[0];
        let biggest_contribution = leader["contributions"]
            .as_u64()
            .ok_or("Failed to retrieve contributions field")?;

        let user_name = leader["login"]
            .as_str()
            .ok_or("Failed to retrieve login field")?;

        let mut bus_factor = biggest_contribution as f64 / total_contributions as f64;

        // println!("bus factor for {name} is {bus_factor}", name=user_name, bus_factor=bus_factor);
        Ok(BusFactor {
            user_name: user_name.to_string(),
            bus_factor,
        })
    }

    /// Returns most popular projects (by stars) for given language in ascending order
    pub fn get_projects(&self, query: &Query) -> Result<(), Box<dyn Error>> {
        let client = reqwest::blocking::Client::new();

        let (_pages, count) = GithubApi::get_pages(query.count);

        let endpoint =
        format!("https://api.github.com/search/repositories?q=language:{language}&sort=stars&order=desc&per_page={per_page}",
          language=query.language, per_page=count);

        let mut res = client
            .get(endpoint)
            .header(USER_AGENT, "bus_factor")
            .basic_auth(&self.username, Some(&self.token))
            .send()?;

        let mut body = String::new();

        res.read_to_string(&mut body)?;

        let body: Value = serde_json::from_str(&body)?;

        for item in body["items"]
            .as_array()
            .ok_or("Failed to get an array of repos")?
        {
            let contributors_url = item["contributors_url"]
                .as_str()
                .ok_or("Failed to get contributors url")?;

            let bus_factor = self.calculate_bus_factor(contributors_url)?;

            if bus_factor.bus_factor >= 0.75 {
                println!(
                    "Project {}, stars {} has bus factor {} for user {}",
                    item["name"],
                    item["stargazers_count"],
                    bus_factor.bus_factor,
                    bus_factor.user_name
                );
            }
        }

        Ok(())
    }
}
