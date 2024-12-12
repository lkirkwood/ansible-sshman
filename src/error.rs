use std::{error::Error, fmt::Display};

#[derive(Debug)]
pub struct UndefinedGroupError {
    pub name: String,
}

impl Error for UndefinedGroupError {}

impl Display for UndefinedGroupError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Undefined section: {}", self.name)
    }
}

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

#[derive(Debug)]
pub struct InvOutputParseError {
    pub message: String,
}

impl Error for InvOutputParseError {}

impl Display for InvOutputParseError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "Failed to parse ansible-inventory output; {}",
            self.message
        )
    }
}
