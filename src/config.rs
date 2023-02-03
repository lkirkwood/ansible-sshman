use serde::{Deserialize, Serialize};
use std::{
    collections::{HashMap, HashSet},
    error::Error,
    hash::Hash,
    iter::FromIterator,
};

use crate::{inventory::Inventory, model::SSHPlay};

/// Models a user in the config file.
#[derive(Debug, Clone, Serialize, Deserialize, Hash, Eq, PartialEq, PartialOrd, Ord)]
pub struct SSHUser {
    pub name: String,
    pub pubkeys: Vec<String>,
    pub access: String,
    pub sudoer: bool,
}

/// Models a config file.
pub struct SSHConfig {
    /// The users defined in the config file.
    pub users: Vec<SSHUser>,
}

impl SSHConfig {
    /// Parses an SSHConfig from some yaml.
    pub fn from_str(content: &str) -> Result<SSHConfig, Box<dyn Error>> {
        return Ok(SSHConfig {
            users: serde_yaml::from_str(content)?,
        });
    }

    /// Applies the config to the provided inventory.
    pub fn apply(&self, inv: &Inventory) -> Result<String, Box<dyn Error>> {
        let mut host_users = HashMap::new();
        for user in &self.users {
            for host in inv.get_path_hosts(&user.access) {
                if !host_users.contains_key(host) {
                    host_users.insert(host, HashSet::new());
                }
                host_users.get_mut(host).unwrap().insert(user.name.clone());
            }
        }

        let mut user_hosts = HashMap::new();
        for (host, users) in host_users {
            let mut user_hash = Vec::from_iter(users);
            user_hash.sort();
            if !user_hosts.contains_key(&user_hash) {
                user_hosts.insert(user_hash.clone(), Vec::new());
            }
            user_hosts.get_mut(&user_hash).unwrap().push(host);
        }
        println!("{user_hosts:#?}");

        let mut plays = Vec::new();
        for (users, hosts) in user_hosts {
            plays.push(SSHPlay::prune_jump_users(hosts.join(":"), users))
        }

        return Ok(serde_yaml::to_string(&plays)?);
    }
}
