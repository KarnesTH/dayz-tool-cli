use serde::{Deserialize, Serialize};
use serde_json::Value;
use thiserror::Error;

pub mod commands;
pub mod utils;

#[derive(Debug, Error, PartialEq)]
pub enum GuidError {
    #[error("Steam64ID must be 17 characters long")]
    InvalidLength,
    #[error("Steam64ID must start with 7656119")]
    InvalidPrefix,
    #[error("Steam64ID must contain only numeric characters")]
    InvalidCharacters,
}

pub type Result<T> = std::result::Result<T, GuidError>;

#[derive(Debug, Error, PartialEq)]
pub enum ConfigError {
    #[error("Failed to create the configuration file")]
    CreateFileError,
    #[error("Failed to read the configuration file")]
    ReadFileError,
    #[error("Failed to write the configuration file")]
    WriteFileError,
    #[error("Failed to parse the configuration file")]
    ParseError,
    #[error("Failed to find the configuration file")]
    OpenFileError,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Root {
    pub profiles: Vec<Profile>,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Profile {
    pub name: String,
    pub workdir_path: String,
    pub workshop_path: String,
    pub installed_mods: Vec<Value>,
}
