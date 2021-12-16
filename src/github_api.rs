use reqwest;
use reqwest::header::USER_AGENT;
use std::io::Read;

pub fn get_repos() -> Result<(), reqwest::Error> {
    let client = reqwest::blocking::Client::new();

    let mut res = client
        .get("https://api.github.com/zen")
        .header(USER_AGENT, "bus_factor")
        .send()?;

    let mut body = String::new();

    res.read_to_string(&mut body);

    println!("Status: {}", res.status());
    println!("Headers:\n{:#?}", res.headers());
    println!("Body:\n{}", body);

    Ok(())
}
