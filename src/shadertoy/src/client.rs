#![allow(dead_code)]

extern crate reqwest;
extern crate serde_json;

use std;
use std::str::FromStr;
use types::*;

#[derive(Serialize, Deserialize, Debug, PartialEq, Copy, Clone)]
pub enum SearchSortOrder {
    Name,
    Love,
    Popular,
    Newest,
    Hot
}

#[derive(Serialize, Deserialize, Debug, PartialEq, Copy, Clone)]
pub enum SearchFilter {
    Vr,
    SoundOutput,
    SoundInput,
    Webcam,
    MultiPass,
    MusicStream
}

/// Search parameters for Client::search.
#[derive(Serialize, Deserialize, Debug, PartialEq, Clone)]
pub struct SearchParams<'a> {
    /// Search string, set as empty to get ALL shadertoys.
    pub string: &'a str,
    /// Sort order of resulting list of shaders.
    pub sort_order: SearchSortOrder,
    /// Inclusion filters, only the shadertoys matching this filter will be included in the result.
    pub filters: Vec<SearchFilter>,
}

/// Client for issuing queries against the Shadertoy API and database
pub struct Client {
    pub api_key: String,
    pub rest_client: reqwest::Client,
}

impl FromStr for SearchSortOrder {
    type Err = ();

    fn from_str(s: &str) -> Result<SearchSortOrder, ()> {
        match s {
            "Name" => Ok(SearchSortOrder::Name),
            "Love" => Ok(SearchSortOrder::Love),
            "Popular" => Ok(SearchSortOrder::Popular),
            "Newest" => Ok(SearchSortOrder::Newest),
            "Hot" => Ok(SearchSortOrder::Hot),
            _ => Err(()),
        }
    }
}

impl FromStr for SearchFilter {
    type Err = ();

    fn from_str(s: &str) -> Result<SearchFilter, ()> {
        match s {
            "VR" => Ok(SearchFilter::Vr),
            "SoundOutput" => Ok(SearchFilter::SoundOutput),
            "SoundInput" => Ok(SearchFilter::SoundInput),
            "Webcam" => Ok(SearchFilter::Webcam),
            "MultiPass" => Ok(SearchFilter::MultiPass),
            "MusicStream" => Ok(SearchFilter::MusicStream),
            _ => Err(()),
        }
    }
}

impl Client {
    /// Create a new client.
    /// This requires sending in an API key, one can generate one on https://www.shadertoy.com/profile
    pub fn new(api_key: &str) -> Client {
        Client {
            api_key: api_key.to_string(),
            rest_client: reqwest::Client::new(),
        }
    }

    /// Issues a search query for shadertoys.
    pub fn search(&self, params: SearchParams) -> Result<Vec<String>, Box<std::error::Error>> {

        let query_str = format!("https://www.shadertoy.com/api/v1/shaders{}?sort={}&{}key={}", 
            match params.string.is_empty() {
                false => format!("/query/{}", params.string),
                true => String::from(""),
            },
            format!("{:?}", params.sort_order).to_lowercase(),
            params.filters.iter().map(|f| 
                format!("filter={:?}&", f).to_lowercase()).collect::<String>(),
            self.api_key);

        println!("{}", &query_str);

        let json_str = self.rest_client.get(&query_str).send()?.text()?;

        #[derive(Serialize, Deserialize, Debug)]
        struct SearchResult {
            #[serde(rename = "Results")]
            results: Vec<String>,
        }

        let search_result: serde_json::Result<SearchResult> = serde_json::from_str(&json_str);

        match search_result {
            Ok(r) => Ok(r.results),
            Err(err) => Err(Box::new(err)),
        }
    }

    pub fn get_shader(&self, shader_id: &str) -> Result<Shader, Box<std::error::Error>> {

        let json_str = self.rest_client
            .get(&format!("https://www.shadertoy.com/api/v1/shaders/{}?key={}", shader_id, self.api_key))
            .send()?
            .text()?;

        #[derive(Serialize, Deserialize, Debug)]
        struct ShaderRoot {
            #[serde(rename = "Shader")]
            shader: Shader,
        }

        let shader_result: serde_json::Result<ShaderRoot> = serde_json::from_str(&json_str);

        match shader_result {
            Ok(r) => Ok(r.shader),
            Err(err) => Err(Box::new(err)),
        }
    }
}
