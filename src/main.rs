mod inventory;
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
    config: String
}

fn main() {
    let args = Args::parse();
    let content = fs::read_to_string(args.inventory).unwrap();
    let inv = inventory::InventoryParser::inv_from_string(content);
    if inv.is_ok() {
        println!("Inventory OK!");
        println!("{:?}", inv);
    } else {
        println!("{}", inv.unwrap_err());
    }
}
