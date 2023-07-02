use super::error::Error;
use super::path_like::PathLike;
use super::result::Result;

use regex::Regex;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct ExpandedHeader {
    line_no: usize,
    path: PathBuf,
}

impl<'a> PathLike<'a> for ExpandedHeader {
    fn path(&'a self) -> &'a Path {
        &self.path
    }
}

impl ExpandedHeader {
    pub fn new<P: AsRef<Path> + ?Sized>(line_no: usize, path: &P) -> Self {
        ExpandedHeader {
            line_no,
            path: path.as_ref().to_path_buf(),
        }
    }

    pub fn line_no(&self) -> &usize {
        &self.line_no
    }

    pub fn is_system(&self) -> bool {
        self.path.starts_with("/usr")
    }

    pub fn parse_line(line: &str) -> Result<Self> {
        let format = Regex::new("^# (\\d+) \"(.*)\"")?;
        return match format.captures(line) {
            Some(matches) => match (matches.get(1), matches.get(2)) {
                (Some(line_no), Some(path)) => match line_no.as_str().parse::<usize>() {
                    Ok(line_no) => Ok(ExpandedHeader::new(line_no, path.as_str())),
                    Err(err) => Err(Error::UsizeParseError(err)),
                },
                _ => Err(Error::LineFormatError(line.to_string())),
            },
            None => Err(Error::LineFormatError(line.to_string())),
        };
    }
}

#[cfg(test)]
mod tests {
    use super::ExpandedHeader;

    #[test]
    fn parse_line() {
        assert_eq!(
            ExpandedHeader::parse_line("# 133 \"/usr/include/stdio.h\" 3 4").unwrap(),
            ExpandedHeader::new(133, "/usr/include/stdio.h")
        );
        assert_eq!(
            ExpandedHeader::parse_line("# 1 \"bad.c\" 2").unwrap(),
            ExpandedHeader::new(1, "bad.c")
        );
    }

    #[test]
    fn is_system() {
        assert_eq!(
            ExpandedHeader::new(1, "/usr/include/stdio.h").is_system(),
            true
        );
        assert_eq!(ExpandedHeader::new(0, "bad.c").is_system(), false);
    }
}
