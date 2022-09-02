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
use std::path::Path;
use std::path::PathBuf;
use std::process::Command;

#[derive(Parser)]
#[clap(author, version, about, long_about = None)]
struct Cli {
    compile_commands: String,
    #[clap(
        long = "preprocessor",
        help = "Replace original source code with preprocessed one"
    )]
    preprocessor: bool,
}

#[derive(Deserialize, Debug)]
#[allow(dead_code)]
struct CompileCommand {
    directory: PathBuf,
    // command: Option<String>,
    arguments: Vec<String>,
    file: PathBuf,
}

fn preprocessor(command: &CompileCommand) -> std::io::Result<()> {
    assert_ne!(command.arguments.len(), 0);
    let mut args = command.arguments.clone();
    struct ReplaceTargetOption {
        c: usize,
        o: usize,
    }
    let replace_target_option: Option<ReplaceTargetOption> =
        (|args: &Vec<String>| -> Option<ReplaceTargetOption> {
            match args.iter().position(|v| v == &String::from("-c")) {
                Some(c) => {
                    let o = args.iter().position(|v| v == &String::from("-o")).unwrap();
                    Some(ReplaceTargetOption { c, o })
                }
                None => None,
            }
        })(&args);
    #[allow(non_snake_case)]
    let option_E = String::from("-E");
    if let Some(replace_target_option) = replace_target_option {
        args[replace_target_option.c] = option_E;
        args.remove(replace_target_option.o + 1);
        args.remove(replace_target_option.o);
    } else {
        args.push(option_E);
    }
    trace!("{:?}", args);

    let output = Command::new(&args[0])
        .args(&args)
        .current_dir(&command.directory)
        .output()?;
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

    let compile_commands = fs::read_to_string(&args.compile_commands)
        .expect(format!("Failed to open file: {:?}", &args.compile_commands).as_str());
    let compile_commands: Vec<CompileCommand> =
        serde_json::from_str(&compile_commands).expect("Failed to parse");
    assert!(compile_commands.len() > 0);
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

            {
                let re = Regex::new(r"([^\w^\(])NULL([^\w^\)])").unwrap();
                apply(&re, &command.file, "$1(NULL)$2");
            }
        })
        .collect();
}
