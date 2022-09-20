use serde::{ser::SerializeMap, Deserialize, Serialize};
use std::{
    collections::{HashMap, HashSet},
    error::Error,
};

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
        for (group, users) in self.get_group_users() {
            plays.push(SSHPlay {
                group: group.to_string(),
                users,
            });
        }
        return Ok(serde_yaml::to_string(&plays)?);
    }
}

/// Models a play that authorizes some users for a group.
struct SSHPlay<'a> {
    /// Path of the group of hosts this play targets.
    group: String,
    /// Maps usernames to public keys
    users: HashSet<&'a SSHUser>,
}

impl<'a> SSHPlay<'a> {
    pub fn tasks(&self) -> Vec<SSHTask> {
        let mut tasks = Vec::new();
        for user in &self.users {
            tasks.extend(SSHTask::tasks_for_user(*user))
        }
        return tasks;
    }
}

impl<'a> Serialize for SSHPlay<'a> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        let mut play = serializer.serialize_map(Some(3))?;
        play.serialize_entry("name", &format!("Set SSH users for {}", &self.group))?;
        play.serialize_entry("hosts", &self.group)?;
        play.serialize_entry("tasks", &self.tasks())?;
        return play.end();
    }
}

/// The various tasks needed to authorize a user on a node.
enum SSHTask {
    /// Creates the user on the node.
    create_user {
        /// Name of user to create.
        name: String,
    },
    /// Authorizes a user's public key on a node.
    authorize_key {
        /// Name of user to authorize.
        name: String,
        /// Public key of user to authorize.
        pubkey: String,
    },
    /// Enables sudo for a user on a node.
    enable_sudo {
        /// Name of user to enable sudo for.
        name: String,
    },
    /// Sets sudo to use the root password.
    use_root_pw_sudo {
        /// Name of user to use root pw with sudo for.
        name: String,
    },
}

impl SSHTask {
    /// Generates the tasks needed to authorize a user on a node.
    pub fn tasks_for_user(user: &SSHUser) -> Vec<Self> {
        return vec![
            Self::create_user {
                name: user.name.clone(),
            },
            Self::authorize_key {
                name: user.name.clone(),
                pubkey: user.pubkey.clone(),
            },
            Self::enable_sudo {
                name: user.name.clone(),
            },
            Self::use_root_pw_sudo {
                name: user.name.clone(),
            },
        ];
    }

    /// Returns the task name.
    fn task_name(&self) -> String {
        match self {
            Self::create_user { name } => format!("Create user {}", name),
            Self::authorize_key { name, pubkey } => format!("Authorize public key for {}", name),
            Self::enable_sudo { name } => format!("Enable sudo for {}", name),
            Self::use_root_pw_sudo { name } => format!("Use root password for sudo for {}", name),
        }
    }

    /// Returns the name of the module used to perform this task.
    fn module_name(&self) -> &'static str {
        match self {
            Self::create_user { name } => return "ansible.builtin.user",
            Self::authorize_key { name, pubkey } => return "ansible.posix.authorized_key",
            _ => return "ansible.builtin.lineinfile",
        }
    }

    /// Returns a map of arguments that configure the module for this task.
    pub fn module_map(&self) -> HashMap<String, String> {
        match self {
            Self::create_user { name } => {
                return HashMap::from([
                    ("name".to_string(), name.clone()),
                    ("state".to_string(), "present".to_string()),
                ])
            }
            Self::authorize_key { name, pubkey } => {
                return HashMap::from([
                    ("key".to_string(), pubkey.clone()),
                    ("user".to_string(), name.clone()),
                    ("manage_dir".to_string(), "true".to_string()),
                ])
            }
            Self::enable_sudo { name } => {
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
            Self::use_root_pw_sudo { name } => {
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
