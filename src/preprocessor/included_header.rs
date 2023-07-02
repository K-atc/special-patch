use super::error::Error;
use super::path_like::PathLike;
use super::result::Result;

use regex::Regex;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct IncludedHeader {
    path: PathBuf,
}

impl<'a> PathLike<'a> for IncludedHeader {
    fn path(&'a self) -> &'a Path {
        &self.path
    }
}

impl IncludedHeader {
    pub fn new<P: AsRef<Path> + ?Sized>(path: &P) -> Self {
        IncludedHeader {
            path: path.as_ref().to_path_buf(),
        }
    }

    pub fn parse_line(line: &str) -> Result<Self> {
        let format = Regex::new("^#\\s*include\\s*[\"<](.*)[\">]")?;
        return match format.captures(line) {
            Some(matches) => match matches.get(1) {
                Some(path) => Ok(IncludedHeader::new(path.as_str())),
                None => Err(Error::LineFormatError(line.to_string())),
            },
            None => Err(Error::LineFormatError(line.to_string())),
        };
    }
}

#[cfg(test)]
mod tests {
    use super::IncludedHeader;
    #[allow(unused_imports)]
    use super::PathLike;

    #[test]
    fn parse_line() {
        assert_eq!(
            IncludedHeader::parse_line("#include <stdio.h>").unwrap(),
            IncludedHeader::new("stdio.h")
        );
        assert_eq!(
            IncludedHeader::parse_line("#\tinclude <stdio.h>").unwrap(),
            IncludedHeader::new("stdio.h")
        );
        assert_eq!(
            IncludedHeader::parse_line("#  include <stdio.h>").unwrap(),
            IncludedHeader::new("stdio.h")
        );
        assert_eq!(
            IncludedHeader::parse_line("#include  <stdio.h>").unwrap(),
            IncludedHeader::new("stdio.h")
        );
        assert_eq!(
            IncludedHeader::parse_line("#include \"trace.h\"").unwrap(),
            IncludedHeader::new("trace.h")
        );
        assert_eq!(
            IncludedHeader::parse_line("#include \"png.h\" /* clang -E -dI */").unwrap(),
            IncludedHeader::new("png.h")
        );
    }

    #[test]
    fn is_system() {
        assert_eq!(
            IncludedHeader::new("/usr/include/stdio.h").is_system(),
            true
        );
        assert_eq!(IncludedHeader::new("bad.c").is_system(), false);
    }
}
