use clap::Parser;
#[allow(unused_imports)]
use log::{info, trace};
#[allow(unused_imports)]
use rayon::prelude::*;
use regex::Regex;
use serde::Deserialize;
use std::ffi::OsStr;
use std::fs;
use std::fs::File;
use std::io::BufWriter;
use std::io::Write;
use std::path::PathBuf;

#[derive(Parser)]
#[clap(author, version, about, long_about = None)]
struct Cli {
    compile_commands: String,
}

#[derive(Deserialize)]
#[allow(dead_code)]
struct CompileCommand {
    directory: PathBuf,
    // command: Option<String>,
    // arguments: Option<String>,
    file: PathBuf,
}

fn main() {
    env_logger::init();

    let args = Cli::parse();

    let compile_commands = fs::read_to_string(&args.compile_commands)
        .expect(format!("Failed to open file: {:?}", &args.compile_commands).as_str());
    let compile_commands: Vec<CompileCommand> =
        serde_json::from_str(&compile_commands).expect("Failed to parse");
    assert!(compile_commands.len() > 0);
    let _: Vec<()> = compile_commands.par_iter().map(|command| {
        trace!("{:?}", command.file);
        if Some("c") != command.file.extension().and_then(OsStr::to_str) {
            return;
        }

        let re = Regex::new(r"([^\w^\(])NULL([^\w^\)])").unwrap();
        let original = fs::read_to_string(&command.file).expect("Failed to read");
        let patched = re.replace_all(original.as_str(), "$1(NULL)$2");
        {
            let file = File::create(&command.file)
                .expect(format!("Failed to open file: {:?}", &args.compile_commands).as_str());
            let mut writer = BufWriter::new(file);
            write!(writer, "{}", patched).expect("Failed to write patched code");
        }
    }).collect();
}
