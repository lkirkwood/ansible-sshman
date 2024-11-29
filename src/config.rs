use serde::{Deserialize, Serialize};
use std::{collections::HashMap, fmt::Display, hash::Hash};

use crate::model::AnsiblePlay;

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
/// Models a user in the config file.
pub struct SSHUser {
    pub name: String,
    pub pubkeys: Vec<String>,
    pub access: HashMap<String, Role>,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(transparent)]
/// Models a config file.
pub struct SSHConfig {
    /// The users defined in the config file.
    pub users: Vec<SSHUser>,
}

impl SSHConfig {
    /// Creates a playbook to apply an SSHConfig.
    pub fn playbook_apply(&self) -> Vec<AnsiblePlay> {
        let mut plays = vec![AnsiblePlay::create_groups()];

        plays.extend(self.users.iter().flat_map(AnsiblePlay::create_user));

        plays.extend(self.users.iter().flat_map(AnsiblePlay::authorize_keys));

        plays
    }
}
