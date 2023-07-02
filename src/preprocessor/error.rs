use std::str::FromStr;

use regex::Error as RegexError;

#[derive(Debug)]
pub enum Error {
    RegexError(RegexError),
    UsizeParseError(<usize as FromStr>::Err),
    LineFormatError(String),
}

impl From<RegexError> for Error {
    fn from(error: RegexError) -> Self {
        Error::RegexError(error)
    }
}
