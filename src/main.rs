mod config;
mod error;
mod inventory;
mod model;

use clap::{Parser, Subcommand};
use inventory::Inventory;
use std::{fs, io::Write, process::Command};
use tempfile::NamedTempFile;

#[derive(Parser, Debug)]
#[clap(author, version, about, long_about = None)]
struct Args {
    /// Path to ssh config file.
    #[clap(short, long, value_parser)]
    config: String,

    /// Ansible inventory. Can be any source accepted by ansible-inventory.
    #[clap(short, long, value_parser)]
    inventory: String,

    /// What to do with the generated playbook.
    #[clap(subcommand)]
    command: Action,
}

/// An action to perform with a playbook.
#[derive(Debug, Subcommand)]
enum Action {
    /// Generates and runs the playbook immediately, with any provided arguments.
    Run {
        /// Extra arguments to pass to ansible-playbook.
        #[clap(last = true)]
        playbook_args: Vec<String>,
    },
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

    let inv_cmd = Command::new("ansible-inventory")
        .args(["-i", &args.inventory])
        .args(["--list", "--export", "--yaml"])
        .output();

    let inv_content = match inv_cmd {
        Ok(output) => match String::from_utf8(output.stdout) {
            Ok(content) => content,
            Err(err) => panic!("Failed to parse inventory output as UTF-8: {err}"),
        },
        Err(err) => panic!("Failed to get output from inventory command: {err}"),
    };

    let inv: Inventory = match serde_yaml::from_str(&inv_content) {
        Ok(inv) => inv,
        Err(err) => panic!("Failed to parse inventory output yaml: {err}"),
    };

    let playbook = conf.apply(&inv).unwrap();

    match args.command {
        Action::Run { playbook_args } => {
            let mut outfile = NamedTempFile::new().expect("Failed to create temp file.");
            outfile
                .write_all(playbook.as_bytes())
                .expect("Failed to write to temp file.");

            std::process::Command::new("ansible-playbook")
                .args(["-i", &args.inventory])
                .arg(outfile.path().to_string_lossy().to_string())
                .args(playbook_args)
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
