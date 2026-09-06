use crate::engine::PummelError;
use crate::models::config::Configuration;
use std::path::Path;

pub mod json;
pub mod toml;
pub mod yaml;

pub trait Parser {
    #[allow(clippy::missing_errors_doc)]
    fn parse<P: AsRef<Path>>(path: P) -> Result<Configuration, PummelError>;
}
