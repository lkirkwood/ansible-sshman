use std::{error::Error, fmt::Display};

#[derive(Debug)]
pub struct InvalidConfigError {
    pub message: String,
}
impl Error for InvalidConfigError {}
impl Display for InvalidConfigError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Invalid SSH config file; {}", self.message)
    }
}
