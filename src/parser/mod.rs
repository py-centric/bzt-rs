use crate::engine::BztError;
use crate::models::config::Configuration;
use std::path::Path;

pub mod json;
pub mod toml;
pub mod yaml;

pub trait Parser {
    fn parse<P: AsRef<Path>>(path: P) -> Result<Configuration, BztError>;
}
