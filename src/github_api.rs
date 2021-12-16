use reqwest;
use reqwest::header::USER_AGENT;
use serde_json::Value;
use std::fs;
use std::io::Read;

//TODO: use enum where applicable
pub struct Query<'a>{
    pub keyword: &'a str,
    pub language: &'a str,
    pub sort: &'a str,
    pub order: &'a str
}
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

    // TODO: use anyhow, or something for err handling
    pub fn get_repos(&self, query: &Query) -> Result<(), reqwest::Error> {


        let client = reqwest::blocking::Client::new();

        // q=tetris+language:assembly&sort=stars&order=desc

        let endpoint =
            format!("https://api.github.com/search/repositories?q={keyword}+language:{language}&sort={sort}&order={order}&per_page=1",
             keyword=query.keyword, language=query.language, sort=query.sort,order=query.order);

        let mut res = client
            // .get("https://api.github.com/search/repositories?q=tetris+language:assembly&sort=stars&order=desc&per_page=5")
            .get(endpoint)
            .header(USER_AGENT, "bus_factor")
            .basic_auth("szymek156", Some(&self.token))
            .send()?;

        let mut body = String::new();

        res.read_to_string(&mut body);

        println!("Status: {}", res.status());

        // TODO: reqwest error, and serde error, how to propagate both?
        let body: Value = serde_json::from_str(&body).unwrap();

        println!("Body:\n{}", body["items"][0]["name"]);

        Ok(())
    }
}
