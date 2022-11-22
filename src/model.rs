use std::collections::HashMap;

use serde::ser::{SerializeMap, SerializeSeq};
use serde::Serialize;

use crate::config::SSHUser;

pub const JUMP_USER_FILE: &'static str = "/home/ansible/.ssh/jump_users";

/// Models an ansible play.
pub struct SSHPlay {
    /// Name of the play.
    pub name: String,
    /// Path of the group of hosts this play targets.
    pub group: String,
    /// Vars to include with the play.
    pub vars: HashMap<String, SSHPlayVars>,
    /// The tasks in this play.
    pub tasks: Vec<SSHTask>,
}

impl SSHPlay {
    /// Convenience function returning a play that deletes all jump users.
    pub fn delete_jump_users() -> SSHPlay {
        let found_var = "found_users";
        let tasks = vec![
            SSHTask::ReadFile {
                path: JUMP_USER_FILE.to_string(),
                var_name: found_var.to_string(),
            },
            SSHTask::DeleteJumpUsers {
                found_var: format!("{}.stdout_lines", found_var),
            },
        ];
        return SSHPlay {
            name: "Removing all jump users".to_string(),
            group: "*".to_string(),
            vars: HashMap::new(),
            tasks,
        };
    }

    /// Convenience function returning a play that deletes the file recording jump users.
    pub fn delete_jump_user_file() -> SSHPlay {
        SSHPlay {
            name: "Removing jump user record file.".to_string(),
            group: "*".to_string(),
            vars: HashMap::new(),
            tasks: vec![SSHTask::DeleteFile {
                path: JUMP_USER_FILE.to_string(),
            }],
        }
    }
}

impl Serialize for SSHPlay {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        let mut play = serializer.serialize_map(Some(3))?;
        play.serialize_entry("name", &self.name)?;
        play.serialize_entry("hosts", &self.group)?;
        play.serialize_entry("vars", &self.vars)?;
        play.serialize_entry("tasks", &self.tasks)?;
        return play.end();
    }
}

/// Models the possible types of vars to include in a play.
pub enum SSHPlayVars {
    String(String),
    List(Vec<String>),
    Dict(HashMap<String, String>),
}

impl Serialize for SSHPlayVars {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        match self {
            Self::String(string) => serializer.serialize_str(string.as_str()),
            Self::List(vec) => {
                let mut seq = serializer.serialize_seq(Some(vec.len()))?;
                for i in vec {
                    seq.serialize_element(i)?;
                }
                seq.end()
            }
            Self::Dict(hmap) => {
                let mut map = serializer.serialize_map(Some(hmap.len()))?;
                for (k, v) in hmap {
                    map.serialize_entry(k, v)?;
                }
                map.end()
            }
        }
    }
}

/// The various tasks needed to authorize a user on a node.
pub enum SSHTask {
    /// Deletes a file on the node.
    DeleteFile {
        path: String,
    },
    /// Reads lines from a file and registers them to the var name.
    /// If file does not exist no error is thrown, var is simply and empty list.
    ReadFile {
        path: String,
        var_name: String,
    },
    ChownDir {
        path: String,
        owner: String,
    },

    /// Creates the user on the node.
    CreateUser {
        /// Name of user to create.
        name: String,
    },
    /// Records a user as a jump user.
    RecordJumpUser {
        /// Name of user to record as jump user.
        name: String,
    },
    /// Deletes all jump users in found_var.
    DeleteJumpUsers {
        /// Var name to read found jump user names from.
        found_var: String,
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
            Self::DeleteFile { path } => {
                format!(
                    "Delete file {}",
                    path.rsplit_once('/').unwrap_or(("", path)).1
                )
            }
            Self::ReadFile { path, var_name } => format!(
                "Read lines of {} into {}",
                path.rsplit_once('/').unwrap_or(("", path)).1,
                var_name
            ),
            Self::ChownDir { path, owner } => format!("Let {} own {}", owner, path),
            Self::CreateUser { name } => format!("Create user {}", name),
            Self::RecordJumpUser { name } => format!("Record jump user {}", name),
            Self::DeleteJumpUsers { found_var } => format!("Deleting users from ${}", found_var),
            Self::AuthorizeKey { name, pubkey: _ } => format!("Authorize public key for {}", name),
            Self::EnableSudo { name } => format!("Enable sudo for {}", name),
            Self::UseRootPWForSudo { name } => format!("Use root password for sudo for {}", name),
        }
    }

    /// Returns the name of the module used to perform this task.
    fn module_name(&self) -> &'static str {
        match self {
            Self::DeleteFile { path: _ } | Self::ChownDir { path: _, owner: _ } => {
                return "ansible.builtin.file"
            }
            Self::ReadFile {
                path: _,
                var_name: _,
            } => return "ansible.builtin.shell",
            Self::CreateUser { name: _ } | Self::DeleteJumpUsers { found_var: _ } => {
                return "ansible.builtin.user"
            }
            Self::AuthorizeKey { name: _, pubkey: _ } => return "ansible.posix.authorized_key",
            Self::RecordJumpUser { name: _ }
            | Self::EnableSudo { name: _ }
            | Self::UseRootPWForSudo { name: _ } => return "ansible.builtin.lineinfile",
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
            Self::ReadFile { path, var_name: _ } => {
                return HashMap::from([(
                    "cmd".to_string(),
                    format!("[ ! -f {} ] || cat {}", path, path),
                )])
            }
            Self::ChownDir { path, owner } => {
                return HashMap::from([
                    ("path".to_string(), path.clone()),
                    ("owner".to_string(), owner.clone()),
                    ("recurse".to_string(), "yes".to_string()),
                ])
            }
            Self::CreateUser { name } => {
                return HashMap::from([
                    ("name".to_string(), name.clone()),
                    ("state".to_string(), "present".to_string()),
                ])
            }
            Self::RecordJumpUser { name } => {
                return HashMap::from([
                    ("path".to_string(), JUMP_USER_FILE.to_string()),
                    ("state".to_string(), "present".to_string()),
                    ("create".to_string(), "yes".to_string()),
                    ("line".to_string(), name.clone()),
                ])
            }
            Self::DeleteJumpUsers { found_var: _ } => {
                return HashMap::from([
                    ("name".to_string(), "{{ item }}".to_string()),
                    ("state".to_string(), "absent".to_string()),
                ])
            }
            Self::AuthorizeKey { name, pubkey } => {
                return HashMap::from([
                    ("key".to_string(), pubkey.clone()),
                    ("comment".to_string(), format!("jump_user: {}", name)),
                    ("user".to_string(), name.clone()),
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

    /// Returns a vector of tasks to be used when setting up a new user.
    pub fn user_tasks(user: &SSHUser) -> Vec<Self> {
        let mut tasks = vec![
            Self::CreateUser {
                name: user.name.clone(),
            },
            Self::RecordJumpUser {
                name: user.name.clone(),
            },
        ];
        for pubkey in &user.pubkeys {
            tasks.push(Self::AuthorizeKey {
                name: user.name.clone(),
                pubkey: pubkey.to_owned(),
            })
        }
        tasks.push(Self::ChownDir {
            path: format!("/home/{}/", user.name.clone()),
            owner: user.name.clone(),
        });
        if user.sudoer == true {
            tasks.push(Self::EnableSudo {
                name: user.name.clone(),
            });
            tasks.push(Self::UseRootPWForSudo {
                name: user.name.clone(),
            });
        }
        return tasks;
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
        match self {
            Self::ReadFile { path: _, var_name } => {
                task.serialize_entry("register", var_name)?;
            }
            Self::DeleteJumpUsers { found_var } => {
                task.serialize_entry("loop", &format!("{{{{ {} }}}}", found_var))?;
                task.serialize_entry("ignore_errors", &true)?;
            }
            _ => {}
        }
        return task.end();
    }
}
