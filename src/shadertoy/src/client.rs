#![allow(dead_code)]

extern crate reqwest;
extern crate serde_json;

use std;
use types::*;

/// Client for issuing queries against the Shadertoy API and database
pub struct Client {
    api_key: String,
    client: reqwest::Client,
}

impl Client {
    /// Create a new client.
    /// This requires sending in an API key, one can generate one on https://www.shadertoy.com/profile
    pub fn new(api_key: &str) -> Client {
        Client {
            api_key: api_key.to_string(),
            client: reqwest::Client::new(),
        }
    }

    /// Issues a search query for shadertoys.
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