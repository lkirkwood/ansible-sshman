use core::str;
use std::{
    collections::{hash_map::Entry, HashMap},
    io::Write,
    path::Path,
    process::Command,
};

use serde_yaml::Value;
use tempfile::NamedTempFile;

use crate::{error::InvOutputParseError, model::AnsiblePlay};

pub fn run_plays(plays: &[AnsiblePlay], args: &[String]) {
    let mut outfile = NamedTempFile::new().expect("Failed to create temp file.");

    outfile
        .write_all(
            serde_yaml::to_string(plays)
                .expect("Failed to serialize playbook.")
                .as_bytes(),
        )
        .expect("Failed to write playbook to temp file.");

    run_playbook(args, outfile.path()).expect("Failed to run playbook");
}

fn run_playbook(args: &[String], path: &Path) -> anyhow::Result<()> {
    Command::new("ansible-playbook")
        .args(args)
        .arg(path)
        .spawn()?
        .wait()?;

    Ok(())
}

/// Returns a list of hosts and their ansible_host var if set.
pub fn list_hosts(pattern: &str) -> anyhow::Result<HashMap<String, Option<String>>> {
    let output = Command::new("ansible-inventory")
        .args(vec!["--list", "--yaml", "--limit", pattern])
        .output()?;

    let mut hosts: HashMap<String, Option<String>> = HashMap::new();

    for hostvar_map in group_hosts(&output.stdout)? {
        for (host_val, vars) in hostvar_map {
            match host_val {
                Value::String(string) => match hosts.entry(string) {
                    Entry::Occupied(mut entry) => {
                        if entry.get().is_none() {
                            entry.insert(hostname_from_vars(vars));
                        }
                    }
                    Entry::Vacant(entry) => {
                        entry.insert(hostname_from_vars(vars));
                    }
                },
                _ => {
                    return Err(InvOutputParseError {
                        message: "Expected string keys in a group listing.".to_string(),
                    }
                    .into())
                }
            }
        }
    }

    Ok(hosts)
}

fn hostname_from_vars(vars: Value) -> Option<String> {
    if let Value::String(hostname) = &vars["ansible_hostname"] {
        Some(hostname.into())
    } else if let Value::String(hostname) = &vars["inventory_hostname"] {
        Some(hostname.into())
    } else if let Value::String(hostname) = &vars["ansible_host"] {
        Some(hostname.into())
    } else if let Value::String(hostname) = &vars["address"] {
        Some(hostname.into())
    } else {
        None
    }
}

/// Transforms the `ansible-inventory --list` output into a list of host->vars mappings.
fn group_hosts(output: &[u8]) -> anyhow::Result<Vec<HashMap<Value, Value>>> {
    let mut maps = vec![];

    match serde_yaml::from_slice(output)? {
        Value::Mapping(root) => {
            if let Some(Value::Mapping(all)) = root.get("all") {
                if let Some(Value::Mapping(children)) = all.get("children") {
                    for (_, group_) in children {
                        match group_ {
                            Value::Mapping(group) => {
                                if let Some(Value::Mapping(group_hosts)) = group.get("hosts") {
                                    maps.push(HashMap::from_iter(
                                        group_hosts
                                            .iter()
                                            .map(|(k, v)| (k.to_owned(), v.to_owned())),
                                    ));
                                }
                            }
                            _ => return Err(InvOutputParseError {
                                message:
                                    "Expected a mapping of hosts to host vars in a group listing."
                                        .to_string(),
                            }
                            .into()),
                        }
                    }
                }
            }
        }
        _ => {
            return Err(InvOutputParseError {
                message: "Expected a mapping from the root of the output.".to_string(),
            }
            .into())
        }
    };

    Ok(maps)
}
