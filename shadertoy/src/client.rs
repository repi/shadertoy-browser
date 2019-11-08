use crate::query::*;
use crate::types::*;
use reqwest;

#[derive(Debug)]
pub enum IssueError {
    Http(reqwest::Error),
    /// Server error
    Server(String),
    /// Query JSON response parsing error
    JsonParse(serde_json::error::Error),
}

impl From<Error> for IssueError {
    fn from(e: Error) -> IssueError {
        match e {
            Error::Server(s) => IssueError::Server(s),
            Error::JsonParse(e) => IssueError::JsonParse(e),
        }
    }
}

impl std::error::Error for IssueError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            IssueError::Http(err) => Some(err),
            IssueError::Server(_) => None,
            IssueError::JsonParse(err) => Some(err),
        }
    }
}

impl std::fmt::Display for IssueError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            IssueError::Http(err) => write!(f, "Failed sending query: {}", err),
            IssueError::Server(msg) => write!(f, "Server error: {}", msg),
            IssueError::JsonParse(err) => write!(f, "Failed parsing JSON server response: {}", err),
        }
    }
}

impl<'a> SearchQuery<'a> {
    pub fn issue(&self, client: &reqwest::Client) -> Result<Vec<String>, IssueError> {
        let json_str = client
            .get(&self.url())
            .send()
            .map_err(IssueError::Http)?
            .text()
            .map_err(IssueError::Http)?;
        Self::process_response(&json_str).map_err(|e| e.into())
    }
}

impl<'a> ShaderQuery<'a> {
    pub fn issue(&self, client: &reqwest::Client) -> Result<Shader, IssueError> {
        let json_str = client
            .get(&self.url())
            .send()
            .map_err(IssueError::Http)?
            .text()
            .map_err(IssueError::Http)?;
        Self::process_response(&json_str).map_err(|e| e.into())
    }
}
