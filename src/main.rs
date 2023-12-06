mod config;
mod error;
mod inventory;
mod model;

use clap::{Parser, Subcommand};
use std::{fs, io::Write};
use tempfile::NamedTempFile;

#[derive(Parser, Debug)]
#[clap(author, version, about, long_about = None)]
struct Args {
    /// Path to ssh config file.
    #[clap(short, long, value_parser)]
    config: String,
    /// Path to Ansible inventory file.
    #[clap(short, long, value_parser)]
    inventory: String,

    #[clap(subcommand)]
    command: Action,
}

#[derive(Debug, Subcommand)]
/// What to do with the generated playbook.
enum Action {
    /// Generates and runs the playbook immediately.
    Run, // TODO add arbitrary args for ansible-playbook
    /// Writes the playbook to a file.
    Write {
        /// Path to write the playbook to.
        #[clap(value_parser)]
        path: String,
    },
}

fn main() {
    let args = Args::parse();
    let conf_content = fs::read_to_string(args.config).expect("Failed to read config file.");
    let conf = config::SSHConfig::from_str(&conf_content).unwrap();

    let inv_content = fs::read_to_string(args.inventory).expect("Failed to read inventory.");
    let inv = inventory::InventoryParser::inv_from_ini(inv_content).unwrap();

    let playbook = conf.apply(&inv).unwrap();

    match args.command {
        Action::Run => {
            let mut outfile = NamedTempFile::new().expect("Failed to create temp file.");
            outfile
                .write_all(playbook.as_bytes())
                .expect("Failed to write to temp file.");

            std::process::Command::new("ansible-playbook")
                .arg(outfile.path().to_string_lossy().to_string())
                .spawn()
                .unwrap()
                .wait()
                .unwrap();

            outfile.close().expect("Failed to remove temp file.");
        }
        Action::Write { path } => {
            fs::write(path, playbook).expect("Failed to write playbook.");
        }
    }
}
