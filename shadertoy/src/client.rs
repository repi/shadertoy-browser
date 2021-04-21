use crate::errors::*;
use crate::types::*;
use reqwest;
use serde::{Deserialize, Serialize};
use serde_json;
use std;
use std::str::FromStr;

#[derive(Serialize, Deserialize, Debug, PartialEq, Copy, Clone)]
pub enum SearchSortOrder {
    Name,
    Love,
    Popular,
    Newest,
    Hot,
}

#[derive(Serialize, Deserialize, Debug, PartialEq, Copy, Clone)]
pub enum SearchFilter {
    Vr,
    SoundOutput,
    SoundInput,
    Webcam,
    MultiPass,
    MusicStream,
}

/// Search parameters for `Client::search`.
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
    pub rest_client: reqwest::blocking::Client,
}

impl FromStr for SearchSortOrder {
    type Err = ();

    fn from_str(s: &str) -> std::result::Result<SearchSortOrder, ()> {
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

    fn from_str(s: &str) -> std::result::Result<SearchFilter, ()> {
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
            rest_client: reqwest::blocking::Client::new(),
        }
    }

    /// Issues a search query for shadertoys.
    /// If the query is successful a list of shader ids will be returned,
    /// which can be used with `get_shader`.
    ///
    /// # Examples
    ///
    /// ```
    /// # fn main() {
    /// let client = shadertoy::Client::new("Bd8tWD"); // insert your own API key here
    /// let search_params = shadertoy::SearchParams {
    /// 	string: "car",
    ///     sort_order: shadertoy::SearchSortOrder::Love,
    ///     filters: vec![],
    /// };
    /// match client.search(&search_params) {
    /// 	Ok(shader_ids) => println!("\"Car\" shadertoys: {:?}", shader_ids),
    /// 	Err(err) => println!("Search failed: {}", err),
    /// }
    /// # }
    /// ```
    pub fn search(&self, params: &SearchParams<'_>) -> Result<Vec<String>> {
        let query_str = format!(
            "https://www.shadertoy.com/api/v1/shaders{}?sort={}&{}key={}",
            if params.string.is_empty() {
                "".to_string()
            } else {
                format!("/query/{}", params.string)
            },
            format!("{:?}", params.sort_order).to_lowercase(),
            params
                .filters
                .iter()
                .map(|f| format!("filter={:?}&", f).to_lowercase())
                .collect::<String>(),
            self.api_key
        );

        let json_str = self.rest_client.get(&query_str).send()?.text()?;

        #[derive(Serialize, Deserialize, Debug)]
        #[serde(deny_unknown_fields)]
        struct SearchResult {
            #[serde(default)]
            #[serde(rename = "Error")]
            error: String,

            #[serde(default)]
            #[serde(rename = "Shaders")]
            shaders: u64,

            #[serde(default)]
            #[serde(rename = "Results")]
            results: Vec<String>,
        }

        match serde_json::from_str::<SearchResult>(&json_str) {
            Ok(r) => {
                if !r.error.is_empty() {
                    bail!("Shadertoy REST search query returned error: {}", r.error);
                }
                Ok(r.results)
            }
            Err(err) => {
                Err(Error::from(err)).chain_err(|| "JSON parsing of Shadertoy search result failed")
            }
        }
    }

    /// Retrives a shader given an id.
    pub fn get_shader(&self, shader_id: &str) -> Result<Shader> {
        let json = self
            .rest_client
            .get(&format!(
                "https://www.shadertoy.com/api/v1/shaders/{}?key={}",
                shader_id, self.api_key
            ))
            .send()?
            .json::<ShaderRoot>()?;

        #[derive(Serialize, Deserialize, Debug)]
        #[serde(deny_unknown_fields)]
        struct ShaderRoot {
            #[serde(default)]
            #[serde(rename = "Error")]
            error: String,

            #[serde(rename = "Shader")]
            shader: Shader,
        }

        if !json.error.is_empty() {
            bail!("Shadertoy REST shader query returned error: {}", json.error);
        }
        Ok(json.shader)
    }
}
