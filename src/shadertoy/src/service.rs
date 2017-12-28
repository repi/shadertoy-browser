#![allow(dead_code)]

extern crate reqwest;
extern crate json;

pub struct Service {
    api_key: String,
    client: reqwest::Client,
}

impl Service {
    fn new(api_key: String) -> Service {
        Service {
            api_key,
            client: reqwest::Client::new(),
        }
    }
}