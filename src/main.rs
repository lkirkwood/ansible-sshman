mod config;
mod error;

use std::fs;

use clap::Parser;

#[derive(Parser, Debug)]
#[clap(author, version, about, long_about = None)]
struct Args {
    /// Path to ansible inventory file.
    #[clap(short, long, value_parser)]
    inventory: String,

    /// Path to ssh config file.
    #[clap(short, long, value_parser)]
    config: String,
}

fn main() {
    let args = Args::parse();
    let conf_content = fs::read_to_string(args.config).expect("Failed to read config file.");
    let conf = config::SSHConfig::from_str(&conf_content).unwrap();

    println!("{}", conf.playbook().unwrap());
    todo!("Removed jump authorized keys before rebuilding file.")
}
