use serde::{Deserialize, Serialize};
use std::{
    collections::{HashMap, HashSet},
    error::Error,
};

use crate::model::{SSHPlay, SSHTask};

#[derive(Debug, Clone, Serialize, Deserialize, Hash, Eq, PartialEq)]
pub struct SSHUser {
    pub name: String,
    pub pubkey: String,
    pub access: String,
}

pub struct SSHConfig {
    pub users: Vec<SSHUser>,
}

impl SSHConfig {
    /// Parses an SSHConfig from some yaml.
    pub fn from_str(content: &str) -> Result<SSHConfig, Box<dyn Error>> {
        return Ok(SSHConfig {
            users: serde_yaml::from_str(content)?,
        });
    }

    /// Maps group paths to the set of users that can access.
    fn get_group_users(&self) -> HashMap<&str, HashSet<&SSHUser>> {
        let mut path_users = HashMap::new();
        for user in &self.users {
            let path = user.access.as_str();
            if !path_users.contains_key(&path) {
                path_users.insert(path, HashSet::new());
            }
            path_users.get_mut(&path).unwrap().insert(user);
        }
        return path_users;
    }

    /// Returns an ansible playbook that applies the settings in this sshconf.
    pub fn playbook(&self) -> Result<String, Box<dyn Error>> {
        let mut plays = Vec::new();
        plays.push(SSHPlay::setup_play());

        for (group, users) in self.get_group_users() {
            let mut tasks = Vec::new();
            for user in users {
                tasks.push(SSHTask::AuthorizeKey {
                    name: user.name.clone(),
                    pubkey: user.pubkey.clone(),
                })
            }

            plays.push(SSHPlay {
                group: group.to_string(),
                tasks,
            });
        }
        return Ok(serde_yaml::to_string(&plays)?);
    }
}
