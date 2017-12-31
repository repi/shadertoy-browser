#![allow(dead_code)]

extern crate reqwest;
extern crate serde_json;

use std;
use types::*;

pub struct Service {
    api_key: String,
    client: reqwest::Client,
}

impl Service {
    pub fn new(api_key: &str) -> Service {
        Service {
            api_key: api_key.to_string(),
            client: reqwest::Client::new(),
        }
    }

    pub fn search(&self, search_str: Option<&str>) -> Result<Vec<String>, Box<std::error::Error>> {

        let query_str: String = {
            if let Some(search_str) = search_str {
                format!("https://www.shadertoy.com/api/v1/shaders/query/{}?key={}", search_str, self.api_key)
            } else {
                format!("https://www.shadertoy.com/api/v1/shaders?key={}", self.api_key)
            }
        };

        let json_str = self.client.get(&query_str).send()?.text()?;

        let search_result: serde_json::Result<SearchResult> = serde_json::from_str(&json_str);

        match search_result {
            Ok(r) => Ok(r.results),
            Err(err) => Err(Box::new(err)),
        }
    }
}
