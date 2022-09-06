use std::{error::Error, fmt::Display};

#[derive(Debug)]
pub struct UndefinedSectionError {
    pub name: String
}
impl Error for UndefinedSectionError {}
impl Display for UndefinedSectionError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Undefined section: {}", self.name)
    }
}