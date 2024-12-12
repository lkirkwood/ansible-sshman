mod config;
mod error;
mod model;
mod modules;
mod plays;
mod subprocess;
#[cfg(test)]
mod tests;

use clap::{Parser, Subcommand};
use config::SSHConfig;
use model::AnsiblePlay;
use std::fs;
use subprocess::run_plays;

#[derive(Parser, Debug)]
#[clap(author, version, about, long_about = None)]
struct Args {
    /// Path to ssh config file.
    #[clap(short, long, value_parser)]
    config: String,

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
    /// Reports on public keys in accounts that aren't configured with sshman.
    Validate {
        /// Extra arguments to pass to ansible-playbook.
        #[clap(last = true)]
        playbook_args: Vec<String>,
    },
    /// Displays a report mapping users to their individual host access.
    Display,
}

fn main() {
    let args = Args::parse();
    let conf_content = fs::read_to_string(&args.config).expect("Failed to read config file.");
    let conf: SSHConfig =
        serde_yaml::from_str(&conf_content).expect("Failed to parse config file.");

    match args.command {
        Action::Run { playbook_args } => run_plays(&conf.create_accounts(), &playbook_args),
        Action::Write { path } => {
            fs::write(
                path,
                serde_yaml::to_string(&conf.create_accounts())
                    .expect("Failed to serialize playbook."),
            )
            .expect("Failed to write playbook.");
        }
        Action::Display => conf.display(),
        Action::Validate { playbook_args } => {
            run_plays(&AnsiblePlay::validate(&conf), &playbook_args)
        }
    }
}
