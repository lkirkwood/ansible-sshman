use crate::{
    error::InvalidConfigError,
    inventory::{Inventory, SectionContainer},
};

use serde::{Deserialize, Serialize};
use std::{
    collections::{HashMap, HashSet},
    error::Error,
};

#[derive(Debug, Serialize, Deserialize, Hash, Eq, PartialEq)]
pub struct SSHUser {
    pub name: String,
    pub pubkey: String,
    pub access: String,
}

pub struct SSHConfig {
    pub users: Vec<SSHUser>,
    pub inv: Inventory,
}

impl SSHConfig {
    pub fn get_hosts(&self, user: &SSHUser) -> Vec<&str> {
        let sec = self.inv.get_by_path(&user.access).unwrap();
        return sec.descended_hosts();
    }

    /// Maps section paths to the set of users that can access.
    fn get_section_users(&self) -> HashMap<String, HashSet<&SSHUser>> {
        let mut path_users = HashMap::new();
        for user in &self.users {
            let full_path = user.access.as_str();
            let mut path_buf = Vec::new();
            for path_comp in full_path.split(':') {
                path_buf.push(path_comp);
                let path = path_buf.join(":");
                if !path_users.contains_key(&path) {
                    path_users.insert(path.clone(), HashSet::new());
                }
                path_users.get_mut(&path).unwrap().insert(user);
            }
        }
        return path_users;
    }

    /// Returns an ansible playbook that applies the settings in this sshconf.
    pub fn playbook(&self) -> Result<String, Box<dyn Error>> {
        let mut outstr = String::new();
        for (path, users) in self.get_section_users() {
            for user in users {
                outstr.push_str(&serde_yaml::to_string(user)?);
            }
        }
        return Ok(outstr);
    }
}

// Parsing

#[derive(Debug)]
pub struct SSHConfigParser;

impl SSHConfigParser {
    pub fn conf_from_string(inv: Inventory, content: String) -> Result<SSHConfig, Box<dyn Error>> {
        let users: Vec<SSHUser> = serde_yaml::from_str(&content)?;
        return Ok(SSHConfig { users, inv });
    }
}
