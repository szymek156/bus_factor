use reqwest;
use reqwest::header::USER_AGENT;
use std::fs;
use std::io::Read;

pub struct GithubApi {
    username: String,
    token: String,
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
            .basic_auth("szymek156", Some(&self.token))
            .send()?;

        let mut body = String::new();

        res.read_to_string(&mut body);

        println!("Status: {}", res.status());
        println!("Body:\n{}", body);

        Ok(())
    }
}
