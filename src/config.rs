use serde::{Deserialize, Serialize};
use std::{
    collections::{HashMap, HashSet},
    error::Error,
};

use crate::model::{SSHPlay, SSHTask, JUMP_USER_FILE};

/// Models a user in the config file.
#[derive(Debug, Clone, Serialize, Deserialize, Hash, Eq, PartialEq)]
pub struct SSHUser {
    pub name: String,
    pub pubkey: String,
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
        let mut plays = Vec::new();
        let mut groups = HashSet::new();
        let mut user_names = HashSet::new();

        let mut usr_plays = Vec::new();
        for (group, users) in self.get_group_users() {
            groups.insert(group.clone());
            user_names.extend::<Vec<String>>(users.iter().map(|usr| usr.name.clone()).collect());

            let mut tasks = Vec::new();
            for user in users {
                tasks.extend(SSHTask::user_tasks(user));
            }
            usr_plays.push(SSHPlay {
                group: group.to_string(),
                vars: HashMap::new(),
                tasks,
            });
        }

        for group in &groups {
            plays.push(SSHPlay::prune_jump_users(
                group.to_string(),
                user_names.clone(),
            ));
        }

        // Insert
        plays.push(SSHPlay {
            group: groups.into_iter().collect::<Vec<&str>>().join(":"),
            vars: HashMap::new(),
            tasks: vec![SSHTask::DeleteFile {
                path: JUMP_USER_FILE.to_string(),
            }],
        });

        plays.extend(usr_plays);
        return Ok(serde_yaml::to_string(&plays)?);
    }
}
