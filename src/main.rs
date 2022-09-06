mod inventory;
mod config;
mod error;

use std::fs;

use clap::Parser;

use crate::inventory::SectionContainer;

#[derive(Parser, Debug)]
#[clap(author, version, about, long_about = None)]
struct Args {
    /// Path to ansible inventory file.
    #[clap(short, long, value_parser)]
    inventory: String,

    /// Path to ssh config file.
    #[clap(short, long, value_parser)]
    config: String
}

fn main() {
    let args = Args::parse();
    let inv_content = fs::read_to_string(args.inventory).expect("Failed to read inventory file.");
    let conf_content = fs::read_to_string(args.config).expect("Failed to read config file.");

    let inv = inventory::InventoryParser::inv_from_string(inv_content).unwrap();
    let conf = config::SSHConfigParser::conf_from_string(inv, conf_content).unwrap();

    for user in &conf.users {
        println!("User {} can access section {} with hosts {:?}", 
            user.name, user.access, conf.get_hosts(&user));
    }
}
