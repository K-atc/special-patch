#![feature(exit_status_error)]

use clap::Parser;
#[allow(unused_imports)]
use log::{info, trace, warn};
#[allow(unused_imports)]
use rayon::prelude::*;
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
use std::path::Path;
use std::path::PathBuf;
use std::process::Command;
use std::process::ExitStatusError;

#[derive(Parser)]
#[clap(author, version, about, long_about = None)]
struct Cli {
    compile_commands: String,
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
    // command: Option<String>,
    arguments: Vec<String>,
    file: PathBuf,
}

#[derive(Debug)]
enum Error {
    IoError(io::Error),
    ExitStatusError(ExitStatusError),
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

fn preprocessor(command: &CompileCommand) -> Result<()> {
    assert_ne!(command.arguments.len(), 0);
    let mut args = command.arguments.clone();
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
    #[allow(non_snake_case)]
    let option_E = String::from("-E");
    if let Some(o) = replace_target_option.o {
        args.remove(o + 1);
        args.remove(o);
    }
    if let Some(c) = replace_target_option.c {
        args[c] = option_E;
    } else {
        args.push(option_E);
    }
    trace!("preprocessor: args={:?}", args);

    let output = Command::new(&args[0])
        .args(&args[1..])
        .current_dir(&command.directory)
        .output()?;
    if !output.stderr.is_empty() {
        let mut stdout = io::stdout().lock();
        stdout.write_all(&output.stderr)?;
    }
    output.status.exit_ok()?;
    assert_ne!(output.stdout.len(), 0);
    let mut patched_file = File::create(&command.file)?;
    patched_file.write_all(&output.stdout)?;
    Ok(())
}

fn apply(re: &Regex, path: &Path, change_to: &str) {
    let original = fs::read_to_string(path).expect("Failed to read");
    let patched = re.replace_all(original.as_str(), change_to);

    {
        let file = File::create(path).expect(format!("Failed to open file: {:?}", path).as_str());
        let mut writer = BufWriter::new(file);
        write!(writer, "{}", patched).expect("Failed to write patched code");
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
                    "Another command for same file. Skip: file={:?}, arguments={:?}",
                    command.file, command.arguments
                );
                continue;
            }
            done_list.insert(&command.file);
            unduplicated_compile_commands.push(command);
        }
        unduplicated_compile_commands
    };

    let _: Vec<()> = compile_commands
        .par_iter()
        .map(|command| {
            trace!("{:?}", command.file);
            if Some("c") != command.file.extension().and_then(OsStr::to_str) {
                return;
            }

            if args.preprocessor {
                preprocessor(command)
                    .expect(format!("Failed to preprocess {:?}", command).as_str());
            }

            if let Some(ref header_name) = args.include {
                let orig = File::open(&command.file)
                    .expect(format!("Failed to open file: {:?}", command.file).as_str());
                let mut reader = BufReader::new(orig);
                let mut buf = Vec::new();
                reader
                    .read_to_end(&mut buf)
                    .expect(format!("Failed to read file: {:?}", command.file).as_str());

                let file = File::create(&command.file)
                    .expect(format!("Failed to create file: {:?}", command.file).as_str());
                let mut writer = BufWriter::new(file);
                write!(writer, "#include <{}>\n", header_name)
                    .expect("Failed to write patched code");
                writer
                    .write_all(buf.as_slice())
                    .expect("Failed to write patched code");
            }

            {
                let re = Regex::new(r"([^\w^\(])NULL([^\w^\)])").unwrap();
                apply(&re, &command.file, "$1(NULL)$2");
            }
        })
        .collect();
}
