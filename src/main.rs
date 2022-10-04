mod config;
mod error;
mod model;

use clap::Parser;
use std::{fs, io::Write};
use tempfile::NamedTempFile;

#[derive(Parser, Debug)]
#[clap(author, version, about, long_about = None)]
struct Args {
    /// Path to ssh config file.
    #[clap(short, long, value_parser)]
    config: String,
}

fn main() {
    let args = Args::parse();
    let conf_content = fs::read_to_string(args.config).expect("Failed to read config file.");
    let conf = config::SSHConfig::from_str(&conf_content).unwrap();

    let mut outfile = NamedTempFile::new().expect("Failed to create temp file.");
    outfile
        .write(conf.playbook().unwrap().as_bytes())
        .expect("Failed to write to temp file.");

    std::process::Command::new("ansible-playbook")
        .arg(outfile.path().to_string_lossy().to_string())
        .spawn()
        .unwrap()
        .wait()
        .unwrap();

    outfile.close().expect("Failed to remove temp file.");
}
