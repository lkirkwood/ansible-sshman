use serde::{Deserialize, Serialize};
use std::{
    collections::{HashMap, HashSet},
    error::Error,
};

use crate::model::{SSHPlay, SSHTask};

/// Models a user in the config file.
#[derive(Debug, Clone, Serialize, Deserialize, Hash, Eq, PartialEq)]
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
        let mut plays = vec![
            SSHPlay::delete_jump_users(),
            SSHPlay::delete_jump_user_file(),
        ];
        let mut groups = HashSet::new();

        let mut usr_plays = Vec::new(); // plays that add users
        for (group, users) in self.get_group_users() {
            groups.insert(group.clone());

            usr_plays.push(SSHPlay {
                name: format!("Add jump users for {}", group),
                group: group.to_string(),
                vars: HashMap::new(),
                tasks: users
                    .iter()
                    .flat_map(|usr| SSHTask::user_tasks(*usr))
                    .collect(),
            });
        }

        plays.extend(usr_plays);
        return Ok(serde_yaml::to_string(&plays)?);
    }
}
