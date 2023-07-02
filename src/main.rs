#![feature(exit_status_error)]

pub mod preprocessor;

use preprocessor::Preprocessor;

use clap::Parser;
#[allow(unused_imports)]
use log::{error, info, trace, warn};
#[allow(unused_imports)]
use rayon::prelude::*;
use regex::Captures;
use regex::Regex;
use serde::Deserialize;
use std::collections::HashSet;
use std::env;
use std::ffi::OsStr;
use std::fs;
use std::fs::File;
use std::io;
use std::io::BufReader;
use std::io::BufWriter;
use std::io::Read;
use std::io::Write;
use std::path::PathBuf;
use std::process::Command;
use std::process::ExitStatusError;

#[derive(Parser)]
#[clap(author, version, about, long_about = None)]
struct Cli {
    #[clap(long = "compile-commands", help = "Path to compile_commands.json")]
    compile_commands: PathBuf,
    files: Vec<PathBuf>,
    #[clap(
        long = "preprocessor",
        help = "Replace original source code with preprocessed one"
    )]
    preprocessor: bool,
    #[clap(long = "include", help = "Add include directive on the top of files")]
    include: Option<String>,
}

#[derive(Deserialize, Debug)]
#[allow(dead_code)]
struct CompileCommand {
    directory: PathBuf,
    #[serde(default)]
    command: Option<String>,
    #[serde(default)]
    arguments: Option<Vec<String>>,
    file: PathBuf,
}

#[derive(Debug)]
enum Error {
    IoError(io::Error),
    ExitStatusError(ExitStatusError),
    ShellWordsParseError(shell_words::ParseError),
    CommandFormatError,
}

type Result<T> = std::result::Result<T, Error>;

impl From<io::Error> for Error {
    fn from(error: io::Error) -> Self {
        Error::IoError(error)
    }
}

impl From<ExitStatusError> for Error {
    fn from(error: ExitStatusError) -> Self {
        Error::ExitStatusError(error)
    }
}

impl From<shell_words::ParseError> for Error {
    fn from(error: shell_words::ParseError) -> Self {
        Error::ShellWordsParseError(error)
    }
}

fn preprocessor(command: &CompileCommand) -> Result<()> {
    let mut args = if let Some(ref arguments) = command.arguments {
        arguments.clone()
    } else if let Some(ref command) = command.command {
        shell_words::split(command)?
    } else {
        return Err(Error::CommandFormatError);
    };
    assert_ne!(args.len(), 0);
    trace!("preprocessor: args={:?}", args);

    #[derive(Debug)]
    struct ReplaceTargetOption {
        c: Option<usize>,
        o: Option<usize>,
    }
    let replace_target_option: ReplaceTargetOption = (|args: &Vec<String>| -> ReplaceTargetOption {
        let o = args.iter().position(|v| v == &String::from("-o"));
        match args.iter().position(|v| v == &String::from("-c")) {
            Some(c) => ReplaceTargetOption { c: Some(c), o },
            None => ReplaceTargetOption { c: None, o },
        }
    })(&args);
    trace!(
        "preprocessor: replace_target_option={:?}",
        replace_target_option
    );
    #[allow(non_snake_case)]
    let option_E = String::from("-E");
    if let Some(c) = replace_target_option.c {
        args[c] = option_E.clone();
    }
    if let Some(o) = replace_target_option.o {
        args.remove(o + 1);
        args.remove(o);
    }
    if replace_target_option.c.is_none() {
        args.push(option_E);
    }

    args.push(String::from("-dI"));

    let output = Command::new(&args[0])
        .args(&args[1..])
        .current_dir(&command.directory)
        .output()?;
    if !output.stderr.is_empty() {
        let mut stdout = io::stdout().lock();
        stdout.write_all(&output.stderr)?;
    }
    output.status.exit_ok()?;

    let patched = String::from_utf8(output.stdout).expect("Invalid UTF-8 sequence");
    assert_ne!(patched.len(), 0);
    let patched = Preprocessor::preprocess(&patched).unwrap();
    let mut patched_file = File::create(&command.file)?;
    patched_file.write_all(&patched.as_bytes())?;

    Ok(())
}

fn open_file(path: &PathBuf) -> String {
    fs::read_to_string(path).expect(format!("Failed to read file: {}", path.display()).as_str())
}

fn save_file(path: &PathBuf, patched: Option<String>) {
    if let Some(patched) = patched {
        let file =
            File::create(path).expect(format!("Failed to open file: {}", path.display()).as_str());
        let mut writer = BufWriter::new(file);
        write!(writer, "{}", patched).expect("Failed to write patched code");
    }
}

fn apply(
    re: &Regex,
    original: String,
    change_to: &str,
    filter: fn(Option<Captures>) -> bool,
) -> Option<String> {
    if filter(re.captures(original.as_str())) {
        return None;
    }
    let patched = re.replace_all(original.as_str(), change_to);
    Some(String::from(patched))
}

fn no_check(caps: Option<Captures>) -> bool {
    if caps.is_none() {
        return true;
    }
    false
}

fn double_quote_exists(caps: Option<Captures>) -> bool {
    if let Some(caps) = caps {
        println!("{:?}", caps);
        caps.get(1).unwrap().as_str().contains("\"") || caps.get(3).unwrap().as_str().contains("\"")
    } else {
        true
    }
}

trait Util {
    fn is_to_be_patched(&self) -> bool;
    fn is_header_file(&self) -> bool;
    fn is_source_file(&self) -> bool;
}

impl Util for PathBuf {
    fn is_to_be_patched(&self) -> bool {
        self.is_header_file() || self.is_source_file()
    }

    fn is_header_file(&self) -> bool {
        match self.extension().and_then(OsStr::to_str) {
            Some("h") | Some("hpp") => true,
            _ => false,
        }
    }

    fn is_source_file(&self) -> bool {
        match self.extension().and_then(OsStr::to_str) {
            Some("c") | Some("cpp") | Some("cc") => true,
            _ => false,
        }
    }
}

fn main() {
    env_logger::init();

    let args = Cli::parse();
    info!("args = {:?}", env::args());

    let compile_commands = fs::read_to_string(&args.compile_commands)
        .expect(format!("Failed to open file: {:?}", &args.compile_commands).as_str());
    let compile_commands: Vec<CompileCommand> =
        serde_json::from_str(&compile_commands).expect("Failed to parse");
    assert!(compile_commands.len() > 0);

    // Filter out commands for same file
    let compile_commands = {
        let mut unduplicated_compile_commands = Vec::new();
        let mut done_list = HashSet::new();
        for command in compile_commands.iter() {
            if done_list.contains(&command.file) {
                warn!(
                    "Another command for same file. Skip: file={:?}, arguments={:?}, command={:?}",
                    command.file, command.arguments, command.command
                );
                continue;
            }
            done_list.insert(&command.file);
            unduplicated_compile_commands.push(command);
        }
        unduplicated_compile_commands
    };

    // Execute compile_commands.json-depend process
    {
        let result: Vec<_> = compile_commands
            .par_iter()
            .map(|command| -> Result<()> {
                trace!("command.file={:?}", command.file);
                if !command.file.is_source_file() {
                    return Ok(());
                }

                // Apply preprocessor
                if args.preprocessor {
                    preprocessor(command)?;
                }

                Ok(())
            })
            .collect();
        let _ = result.iter().map(|v| {
            v.as_ref().map_err(|err| {
                error!(
                    "Failed to preprocess files in compile_commands.json: {:?}",
                    err
                )
            })
        });
        let _ = result.iter().map(|v| {
            v.as_ref()
                .expect("Failed to process files in compile_commands.json")
        });
    }

    let files_from_compile_commands: HashSet<PathBuf> = compile_commands
        .iter()
        .map(|v| v.file.clone())
        .filter(|v| v.is_to_be_patched())
        .collect();

    for file_path in files_from_compile_commands.iter().chain(args.files.iter()) {
        trace!("file_path={:?}", file_path);

        // Insert include file
        if let Some(ref header_name) = args.include {
            let orig = File::open(file_path)
                .expect(format!("Failed to open file: {:?}", file_path).as_str());
            let mut reader = BufReader::new(orig);
            let mut buf = Vec::new();
            reader
                .read_to_end(&mut buf)
                .expect(format!("Failed to read file: {:?}", file_path).as_str());

            let file = File::create(&file_path)
                .expect(format!("Failed to create file: {:?}", file_path).as_str());
            let mut writer = BufWriter::new(file);
            write!(writer, "#include <{}>\n", header_name).expect("Failed to write patched code");
            writer
                .write_all(buf.as_slice())
                .expect("Failed to write patched code");
        }

        // Wrap NULL with brackets
        {
            let re = Regex::new(r"([^\w^\(])NULL([^\w^\)])").unwrap();
            let patched = apply(&re, open_file(file_path), "$1(NULL)$2", no_check);
            save_file(file_path, patched);
        }

        // Wipeout constexpr functions
        {
            // Un-constexpr functions
            let re = Regex::new(r"constexpr\s(.*(\r)?(\n)?(\s*)\{)").unwrap();
            let patched = apply(&re, open_file(file_path), "$1", no_check);
            save_file(file_path, patched);

            // Un-constexpr objects
            if file_path.is_source_file() {
                let re = Regex::new(r"static constexpr\s(.*;)").unwrap();
                let patched = apply(&re, open_file(file_path), "$1", no_check);
                save_file(file_path, patched);
            }
        }

        // Escape single quotes in const char for yaml string
        {
            let patched = escape_single_quotes_in_const_char(open_file(file_path));
            save_file(file_path, patched);
        }
    }
}

fn escape_single_quotes_in_const_char(original: String) -> Option<String> {
    let re = Regex::new("\"(.*?)\\\\?'([^\"\n]{2,}?)\\\\?'(.*?)\"").unwrap();
    apply(&re, original, "\"$1''$2''$3\"", double_quote_exists)
}

#[cfg(test)]
mod tests {
    use super::*;

    macro_rules! test_case {
        ($original:expr, $patched:expr) => {
            assert_eq!(
                escape_single_quotes_in_const_char(String::from($original)),
                $patched
            );
        };
    }

    #[test]
    fn single_quote() {
        test_case!(
            "let test = \"test 'ab'.\"",
            Some(String::from("let test = \"test ''ab''.\""))
        );
    }

    #[test]
    fn single_quote_confusion() {
        test_case!("{\"text\", OPT_TEXT, '-', \"Print as text\"}", None);
        test_case!(
            "{\"select\", OPT_SELECT_NAME, 's', \"Select a single algorithm\"},",
            None
        );
        test_case!("{ OPT_SECTION_STR, 1, '-', \"Random state\" \" options:\n\" }, {\"rand\", OPT_R_RAND, 's', \"Load the given file(s) into the random number generator\"}, {\"writerand\", OPT_R_WRITERAND, '>', \"Write random data to the specified file\"}", None);

        test_case!("\"\n'abc'\"", None);
        test_case!("\"'ab\nc'\"", None);
    }
}
