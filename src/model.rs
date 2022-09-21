use std::collections::HashMap;

use serde::ser::SerializeMap;
use serde::Serialize;

const JUMP_USER_NAME: &'static str = "jump";

/// Models an ansible play.
pub struct SSHPlay {
    /// Path of the group of hosts this play targets.
    pub group: String,
    /// The tasks in this play.
    pub tasks: Vec<SSHTask>,
}

impl SSHPlay {
    pub fn setup_play() -> SSHPlay {
        return SSHPlay {
            group: "*".to_string(),
            tasks: vec![
                SSHTask::CreateUser {
                    name: JUMP_USER_NAME.to_string(),
                },
                SSHTask::DeleteFile {
                    path: format!("/home/{}/.ssh/authorized_keys", JUMP_USER_NAME),
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
pub enum SSHTask {
    /// Deletes a file on the node.
    DeleteFile { path: String },
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
            Self::DeleteFile { path } => {
                format!(
                    "Delete file {}",
                    path.rsplit_once('/').unwrap_or(("", path)).1
                )
            }
            Self::AuthorizeKey { name, pubkey: _ } => format!("Authorize public key for {}", name),
            Self::EnableSudo { name } => format!("Enable sudo for {}", name),
            Self::UseRootPWForSudo { name } => format!("Use root password for sudo for {}", name),
        }
    }

    /// Returns the name of the module used to perform this task.
    fn module_name(&self) -> &'static str {
        match self {
            Self::DeleteFile { path: _ } => return "ansible.builtin.file",
            Self::CreateUser { name: _ } => return "ansible.builtin.user",
            Self::AuthorizeKey { name: _, pubkey: _ } => return "ansible.posix.authorized_key",
            _ => return "ansible.builtin.lineinfile",
        }
    }

    /// Returns a map of arguments that configure the module for this task.
    pub fn module_map(&self) -> HashMap<String, String> {
        match self {
            Self::DeleteFile { path } => {
                return HashMap::from([
                    ("path".to_string(), path.clone()),
                    ("state".to_string(), "absent".to_string()),
                ])
            }
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
