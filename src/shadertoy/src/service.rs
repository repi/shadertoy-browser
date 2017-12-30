#![allow(dead_code)]

extern crate reqwest;
extern crate json;

pub struct Service {
    api_key: String,
    client: reqwest::Client,
}

impl Service {
    fn new(api_key: &str) -> Service {
        Service {
            api_key: api_key.to_string(),
            client: reqwest::Client::new(),
        }
    }
}