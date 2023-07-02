pub mod error;
pub mod expanded_header;
pub mod included_header;
pub mod path_like;
pub mod result;

use std::{
    collections::{HashMap, HashSet},
    path::PathBuf,
};

use error::Error;
use expanded_header::ExpandedHeader;
use included_header::IncludedHeader;
use path_like::PathLike;
use result::Result;

#[derive(Debug, PartialEq, Eq)]
pub enum Line {
    IncludedHeader(IncludedHeader),
    ExpandedHeader(ExpandedHeader),
    Other(String),
}

enum SourceContext {
    IncludedHeader(IncludedHeader),
    ExpandedHeader(ExpandedHeader),
    Main,
}

#[derive(Debug)]
pub struct Preprocessor {}

impl Preprocessor {
    pub fn preprocess(source: &str) -> Result<String> {
        let mut current_source_context = SourceContext::Main;
        let mut markers: HashMap<usize, ExpandedHeader> = HashMap::new();
        let mut expands: HashSet<PathBuf> = HashSet::new();
        for line in source.lines() {
            match Self::parse_line(line)? {
                Line::ExpandedHeader(ref header) => {
                    current_source_context = SourceContext::ExpandedHeader(header.clone());
                    expands.insert(header.path().to_path_buf());
                }
                Line::IncludedHeader(_) => {
                    if let SourceContext::ExpandedHeader(ref marker) = current_source_context {
                        markers.insert(*marker.line_no(), marker.clone());
                    }
                }
                _ => (),
            }
        }

        let mut current_source_context = SourceContext::Main;
        let mut result = String::new();
        'outer: for line in source.lines() {
            match Self::parse_line(line)? {
                Line::ExpandedHeader(ref header) => {
                    current_source_context = SourceContext::ExpandedHeader(header.clone());
                    // `error: invalid line marker flag '2': cannot pop empty include stack` を避けるため、出力はする
                    // if header.is_system() {
                    //     continue;
                    // }
                }
                Line::IncludedHeader(ref header) => {
                    if let SourceContext::ExpandedHeader(ref last_expand) = current_source_context {
                        if last_expand.is_system() {
                            continue;
                        }
                    }
                    for path in expands.iter() {
                        // Ignore include statement that already expanded
                        if !path.is_system() && path.ends_with(header.path()) {
                            continue 'outer;
                        }
                    }
                }
                Line::Other(_) => {
                    if let SourceContext::ExpandedHeader(ref header) = current_source_context {
                        if header.is_system() {
                            continue;
                        }
                    }
                }
            }
            result = result + line + "\n";
        }
        Ok(result)
    }

    fn parse_line(line: &str) -> Result<Line> {
        {
            let result = IncludedHeader::parse_line(line);
            match result {
                Ok(result) => return Ok(Line::IncludedHeader(result)),
                Err(err) => match err {
                    Error::LineFormatError(_) => {}
                    err => return Err(err),
                },
            }
        }
        {
            let result = ExpandedHeader::parse_line(line);
            match result {
                Ok(result) => return Ok(Line::ExpandedHeader(result)),
                Err(err) => match err {
                    Error::LineFormatError(_) => {}
                    err => return Err(err),
                },
            }
        }
        return Ok(Line::Other(line.to_string()));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_line() {
        assert_eq!(
            Preprocessor::parse_line("# 46 \"/usr/lib/llvm-16/lib/clang/16/include/stddef.h\" 3 4")
                .unwrap(),
            Line::ExpandedHeader(ExpandedHeader::new(
                46,
                "/usr/lib/llvm-16/lib/clang/16/include/stddef.h"
            ))
        );
        assert_eq!(
            Preprocessor::parse_line("#include <trace.h>").unwrap(),
            Line::IncludedHeader(IncludedHeader::new("trace.h"))
        );
        assert_eq!(
            Preprocessor::parse_line("").unwrap(),
            Line::Other(String::from(""))
        );
        assert_eq!(
            Preprocessor::parse_line("typedef unsigned long int __uint64_t;").unwrap(),
            Line::Other(String::from("typedef unsigned long int __uint64_t;"))
        );
    }

    #[test]
    fn preprocess() {
        let source = std::fs::read_to_string("./test/preprocessor/main.E").unwrap();
        let exptected = std::fs::read_to_string("./test/preprocessor/main.E.expected").unwrap();
        let result = Preprocessor::preprocess(&source).unwrap();
        if result != exptected {
            println!("{}", difference::Changeset::new(&exptected, &result, "\n"));
        }
        assert!(result == exptected);
    }

    // #[test]
    // fn debug() {
    //     let source = std::fs::read_to_string("./test/preprocessor/pngrtran.c").unwrap();
    //     for line in source.lines() {
    //         println!("{:?}", Preprocessor::parse_line(line));
    //     }
    //     println!("{}", Preprocessor::preprocess(&source).unwrap());
    //     assert!(false)
    // }

    #[test]
    fn path() {
        let header1 = ExpandedHeader::new(0, "/source/magma/targets/libpng/repo/png.h");
        let header2 = IncludedHeader::new("png.h");
        assert!(header1.path().ends_with(header2.path()));
    }
}
