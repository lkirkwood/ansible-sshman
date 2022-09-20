use serde::{ser::SerializeMap, Deserialize, Serialize};
use std::{
    collections::{HashMap, HashSet},
    error::Error,
};

const JUMP_USER_NAME: &'static str = "jump";

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

/// Models an ansible play.
struct SSHPlay {
    /// Path of the group of hosts this play targets.
    group: String,
    /// The tasks in this play.
    tasks: Vec<SSHTask>,
}

impl SSHPlay {
    fn setup_play() -> SSHPlay {
        return SSHPlay {
            group: "*".to_string(),
            tasks: vec![
                SSHTask::CreateUser {
                    name: JUMP_USER_NAME.to_string(),
                },
                SSHTask::EnableSudo {
                    name: JUMP_USER_NAME.to_string(),
                },
                SSHTask::UseRootPWForSudo {
                    name: JUMP_USER_NAME.to_string(),
                },
            ],
        };
    }
}

impl Serialize for SSHPlay {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        let mut play = serializer.serialize_map(Some(3))?;
        play.serialize_entry("name", &format!("Set SSH users for {}", &self.group))?;
        play.serialize_entry("hosts", &self.group)?;
        play.serialize_entry("tasks", &self.tasks)?;
        return play.end();
    }
}

/// The various tasks needed to authorize a user on a node.
enum SSHTask {
    /// Creates the user on the node.
    CreateUser {
        /// Name of user to create.
        name: String,
    },
    /// Authorizes a user's public key on a node.
    AuthorizeKey {
        /// Name of user to authorize.
        name: String,
        /// Public key of user to authorize.
        pubkey: String,
    },
    /// Enables sudo for a user on a node.
    EnableSudo {
        /// Name of user to enable sudo for.
        name: String,
    },
    /// Sets sudo to use the root password.
    UseRootPWForSudo {
        /// Name of user to use root pw with sudo for.
        name: String,
    },
}

impl SSHTask {
    /// Returns the task name.
    fn task_name(&self) -> String {
        match self {
            Self::CreateUser { name } => format!("Create user {}", name),
            Self::AuthorizeKey { name, pubkey: _ } => format!("Authorize public key for {}", name),
            Self::EnableSudo { name } => format!("Enable sudo for {}", name),
            Self::UseRootPWForSudo { name } => format!("Use root password for sudo for {}", name),
        }
    }

    /// Returns the name of the module used to perform this task.
    fn module_name(&self) -> &'static str {
        match self {
            Self::CreateUser { name: _ } => return "ansible.builtin.user",
            Self::AuthorizeKey { name: _, pubkey: _ } => return "ansible.posix.authorized_key",
            _ => return "ansible.builtin.lineinfile",
        }
    }

    /// Returns a map of arguments that configure the module for this task.
    pub fn module_map(&self) -> HashMap<String, String> {
        match self {
            Self::CreateUser { name } => {
                return HashMap::from([
                    ("name".to_string(), name.clone()),
                    ("state".to_string(), "present".to_string()),
                ])
            }
            Self::AuthorizeKey { name, pubkey } => {
                return HashMap::from([
                    ("key".to_string(), pubkey.clone()),
                    ("comment".to_string(), format!("jump_user: {}", name)),
                    ("user".to_string(), JUMP_USER_NAME.to_string()),
                    ("manage_dir".to_string(), "true".to_string()),
                ])
            }
            Self::EnableSudo { name } => {
                return HashMap::from([
                    (
                        "path".to_string(),
                        "/etc/sudoers.d/ansible-sshman".to_string(),
                    ),
                    ("state".to_string(), "present".to_string()),
                    ("create".to_string(), "yes".to_string()),
                    ("line".to_string(), format!("{} ALL = (ALL) ALL", name)),
                ])
            }
            Self::UseRootPWForSudo { name } => {
                return HashMap::from([
                    (
                        "path".to_string(),
                        "/etc/sudoers.d/ansible-sshman".to_string(),
                    ),
                    ("state".to_string(), "present".to_string()),
                    ("create".to_string(), "yes".to_string()),
                    ("line".to_string(), format!("Defaults:{} rootpw", name)),
                ])
            }
        }
    }
}

impl Serialize for SSHTask {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        let mut task = serializer.serialize_map(Some(3))?;
        task.serialize_entry("name", &self.task_name())?;
        task.serialize_entry("become", &true)?;
        task.serialize_entry(self.module_name(), &self.module_map())?;
        return task.end();
    }
}
