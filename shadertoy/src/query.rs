use crate::types::*;
use serde::{Deserialize, Serialize};
use serde_json;
use std;
use std::result::Result;
use std::str::FromStr;

/// Root URL that all http queries will be used on
static ROOT_URL: &str = "https://www.shadertoy.com/api/v1/";

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

#[derive(Debug)]
pub enum Error {
    /// Server error
    Server(String),
    /// Query JSON response parsing error
    JsonParse(serde_json::error::Error),
}

impl std::error::Error for Error {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Error::Server(_) => None,
            Error::JsonParse(err) => Some(err),
        }
    }
}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Error::Server(msg) => write!(f, "Server error: {}", msg),
            Error::JsonParse(err) => write!(f, "Failed parsing JSON server response: {}", err),
        }
    }
}

/// Shadertoy string search query
#[derive(Serialize, Deserialize, Debug, PartialEq, Clone)]
pub struct SearchQuery<'a> {
    /// Search string, set as empty to get ALL shadertoys.
    pub string: &'a str,
    /// Sort order of resulting list of shaders.
    pub sort_order: SearchSortOrder,
    /// Inclusion filters, only the shadertoys matching this filter will be included in the result.
    pub filters: Vec<SearchFilter>,
    /// Shadertoy user API key
    pub api_key: &'a str,
}

impl<'a> SearchQuery<'a> {
    pub fn url(&self) -> String {
        format!(
            "{}shaders{}?sort={}&{}key={}",
            ROOT_URL,
            if self.string.is_empty() {
                "".to_string()
            } else {
                format!("/query/{}", self.string)
            },
            format!("{:?}", self.sort_order).to_lowercase(),
            self.filters
                .iter()
                .map(|f| format!("filter={:?}&", f).to_lowercase())
                .collect::<String>(),
            self.api_key
        )
    }

    pub fn process_response(text: &str) -> Result<Vec<String>, Error> {
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
        match serde_json::from_str::<SearchResult>(&text) {
            Ok(r) => {
                if !r.error.is_empty() {
                    return Err(Error::Server(r.error));
                }
                Ok(r.results)
            }
            Err(err) => Err(Error::JsonParse(err)),
        }
    }
}

/// Shadertoy shader retrieve query
#[derive(Serialize, Deserialize, Debug, PartialEq, Clone)]
pub struct ShaderQuery<'a> {
    /// ID  for shader to retrieve
    pub shader_id: &'a str,
    /// Shadertoy user API key
    pub api_key: &'a str,
}

impl<'a> ShaderQuery<'a> {
    pub fn url(&self) -> String {
        format!(
            "{}shaders/{}?key={}",
            ROOT_URL, self.shader_id, self.api_key
        )
    }

    pub fn process_response(text: &str) -> Result<Shader, Error> {
        #[derive(Serialize, Deserialize, Debug)]
        #[serde(deny_unknown_fields)]
        struct ShaderRoot {
            #[serde(default)]
            #[serde(rename = "Error")]
            error: String,
            #[serde(rename = "Shader")]
            shader: Shader,
        }
        match serde_json::from_str::<ShaderRoot>(&text) {
            Ok(r) => {
                if !r.error.is_empty() {
                    return Err(Error::Server(r.error));
                }
                Ok(r.shader)
            }
            Err(err) => Err(Error::JsonParse(err)),
        }
    }
}
