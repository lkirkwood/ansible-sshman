use serde::{Deserialize, Serialize};
use std::{collections::HashMap, fmt::Display, hash::Hash, process::exit};

use crate::{model::AnsiblePlay, subprocess};

#[derive(Debug, Clone, Serialize, Deserialize, Hash, Eq, PartialEq, PartialOrd, Ord)]
#[serde(rename_all = "lowercase")]
pub enum Role {
    Blocked,
    Sudoer,
    Nopass,
    SuperUser,
}

impl Role {
    /// Returns the name of the group for a user with this role.
    pub fn group(&self) -> &'static str {
        match self {
            Self::Blocked => "sshman-blocked",
            Self::Sudoer => "sshman-sudoer",
            Self::Nopass => "sshman-nopass",
            Self::SuperUser => "root",
        }
    }
}

impl Display for Role {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Blocked => write!(f, "blocked user"),
            Self::Sudoer => write!(f, "sudo user"),
            Self::Nopass => write!(f, "passwordless sudo user"),
            Self::SuperUser => write!(f, "super user"),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Eq, PartialEq)]
pub struct AccessStmt {
    pub hosts: String,
    pub role: Role,
    #[serde(default)]
    pub groups: Vec<String>,
    pub seuser: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Eq, PartialEq)]
/// Models a user in the config file.
pub struct SSHUser {
    pub name: String,
    pub pubkeys: Vec<String>,
    pub access: Vec<AccessStmt>,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(transparent)]
/// Models a config file.
pub struct SSHConfig {
    /// The users defined in the config file.
    pub users: Vec<SSHUser>,
}

impl SSHConfig {
    /// Creates a playbook to create accounts.
    pub fn create_accounts(&self) -> Vec<AnsiblePlay> {
        let mut plays = vec![AnsiblePlay::create_groups(
            self.users
                .iter()
                .flat_map(|usr| &usr.access)
                .flat_map(|access| access.groups.clone()),
        )];

        plays.extend(self.users.iter().flat_map(AnsiblePlay::create_user));

        plays.extend(self.users.iter().flat_map(AnsiblePlay::authorize_keys));

        plays
    }

    pub fn display(&self) {
        let mut pattern_hosts = HashMap::new();

        for user in &self.users {
            println!("# User: {}", user.name);
            for stmt in &user.access {
                println!("  host pattern: {}", stmt.hosts);
                println!("  role: {}", stmt.role);

                if !stmt.groups.is_empty() {
                    println!("  groups: {}", stmt.groups.join(", "));
                }

                if let Some(seuser) = &stmt.seuser {
                    println!("  seuser: {seuser}");
                }

                let hosts = if let Some(hosts_) = pattern_hosts.get(&stmt.hosts) {
                    hosts_
                } else {
                    match subprocess::list_hosts(&stmt.hosts) {
                        Ok(hosts_) => {
                            pattern_hosts.insert(&stmt.hosts, hosts_);
                            pattern_hosts.get(&stmt.hosts).unwrap()
                        }
                        Err(err) => {
                            println!("{err}");
                            exit(1)
                        }
                    }
                };

                println!("\n## Hosts:");
                for (host, hostname_) in hosts {
                    print!("  + {host}");
                    if let Some(hostname) = hostname_ {
                        print!(" -- ({hostname})");
                    }
                    println!();
                }
                println!();
            }
        }
    }
}
