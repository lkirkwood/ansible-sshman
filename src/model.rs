use std::collections::HashMap;

use serde::ser::{SerializeMap, SerializeSeq};
use serde::Serialize;

use crate::config::{Role, SSHUser};

pub const JUMP_USER_FILE: &str = "/home/ansible/.ssh/jump_users";

/// Models an ansible play.
#[derive(Debug)]
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
    /// Convenience function returning a play that removes old users and adds new ones.
    pub fn set_accounts(group: String, allowed: Vec<&SSHUser>) -> SSHPlay {
        let found_users_var = "found_users".to_string();
        let all_users_var = "allowed_users".to_string();
        let normal_users_var = "normal_users".to_string();
        let sudo_users_var = "sudo_users".to_string();
        let super_users_var = "super_users".to_string();

        let (mut all, mut users, mut sudoers, mut superusers) = (vec![], vec![], vec![], vec![]);
        for user in allowed {
            all.push(user.name.clone());
            match user.role {
                Role::User => users.push(user.name.clone()),
                Role::Sudoer => sudoers.push(user.name.clone()),
                Role::SuperUser => superusers.push(user.name.clone()),
            }
        }

        SSHPlay {
            name: "Updating jump users".to_string(),
            group,
            vars: HashMap::from([
                (all_users_var.clone(), SSHPlayVars::List(all)),
                (normal_users_var.clone(), SSHPlayVars::List(users)),
                (sudo_users_var.clone(), SSHPlayVars::List(sudoers)),
                (super_users_var.clone(), SSHPlayVars::List(superusers)),
            ]),
            tasks: vec![
                SSHTask::ReadFile {
                    path: JUMP_USER_FILE.to_string(),
                    var_name: found_users_var.clone(),
                },
                SSHTask::PruneUsers {
                    found_var: found_users_var.clone(),
                    allowed_var: all_users_var.clone(),
                },
                SSHTask::AddUsers {
                    found_var: found_users_var.clone(),
                    allowed_var: normal_users_var,
                },
                SSHTask::AddSudoers {
                    found_var: found_users_var.clone(),
                    allowed_var: sudo_users_var,
                },
                SSHTask::AddSuperUsers {
                    found_var: found_users_var.clone(),
                    allowed_var: super_users_var,
                },
            ],
        }
    }

    /// Convenience function returning a play that authenticates existing users.
    pub fn set_user_pubkeys(group: String, users: Vec<&SSHUser>) -> SSHPlay {
        let mut tasks = Vec::new();
        for user in users {
            let keys = user.pubkeys.join("\n");
            tasks.push(SSHTask::SetKeys {
                user: user.name.clone(),
                keys,
            });

            if let Role::Sudoer = user.role {
                tasks.push(SSHTask::EnableSudo {
                    name: user.name.clone(),
                });
            }
        }

        SSHPlay {
            name: "Authenticating users".to_string(),
            group,
            vars: HashMap::new(),
            tasks,
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
        play.end()
    }
}

/// Models the possible types of vars to include in a play.
#[allow(unused)]
#[derive(Debug)]
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
#[allow(unused)]
#[derive(Debug)]
pub enum SSHTask {
    /// Deletes a file on the node.
    DeleteFile { path: String },
    /// Reads lines from a file and registers them to the var name.
    /// If file does not exist no error is thrown, var is simply and empty list.
    ReadFile { path: String, var_name: String },
    ChownDir {
        path: String,
        owner: String,
        group: Option<String>,
    },

    /// Creates the user on the node.
    CreateUser {
        /// Name of user to create.
        name: String,
    },
    /// Records a user as a user.
    RecordJumpUser {
        /// Name of user to record as user.
        names: Vec<String>,
    },
    /// Deletes all users in found_var that are not present in allowed_var.
    PruneUsers {
        /// Var name to read found user names from.
        found_var: String,
        /// Var name to read allowed user names from.
        allowed_var: String,
    },
    /// Creates users in allowed_var not present in found_var.
    AddUsers {
        /// Var name to read found user names from.
        found_var: String,
        /// Var name to read allowed user names from.
        allowed_var: String,
    },
    AddSudoers {
        /// Var name to read found user names from.
        found_var: String,
        /// Var name to read allowed user names from.
        allowed_var: String,
    },
    AddSuperUsers {
        /// Var name to read found user names from.
        found_var: String,
        /// Var name to read allowed user names from.
        allowed_var: String,
    },
    /// Sets the public keys a user can use to authenticate, if the value of the variable named by matched is false..
    SetKeys {
        /// User to set the public keys for.
        user: String,
        /// Keys to authorize for the user.
        keys: String,
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
            Self::ChownDir { path, owner, group } => format!("Let {owner}:{group:?} own {path}"),
            Self::CreateUser { name } => format!("Create user {}", name),
            Self::RecordJumpUser { names } => format!("Record jump user {names:?}"),
            Self::PruneUsers {
                found_var,
                allowed_var,
            } => format!("Deleting users from ${found_var} not present in ${allowed_var}"),
            Self::AddUsers {
                found_var,
                allowed_var,
            } => format!("Adding users from ${allowed_var} not present in ${found_var}"),
            Self::AddSudoers {
                found_var,
                allowed_var,
            } => format!("Adding sudoers from ${allowed_var} not present in ${found_var}"),
            Self::AddSuperUsers {
                found_var,
                allowed_var,
            } => format!("Adding superusers from ${allowed_var} not present in ${found_var}"),
            Self::SetKeys { user, .. } => {
                format!("Setting public keys for {user}")
            }
            Self::EnableSudo { name } => format!("Enable sudo for {}", name),
            Self::UseRootPWForSudo { name } => format!("Use root password for sudo for {}", name),
        }
    }

    /// Returns the name of the module used to perform this task.
    fn module_name(&self) -> &'static str {
        match self {
            Self::DeleteFile { .. } | Self::RecordJumpUser { .. } | Self::ChownDir { .. } => {
                "ansible.builtin.file"
            }

            Self::ReadFile { .. } => "ansible.builtin.shell",

            Self::CreateUser { .. }
            | Self::PruneUsers { .. }
            | Self::AddUsers { .. }
            | Self::AddSudoers { .. }
            | Self::AddSuperUsers { .. } => "ansible.builtin.user",

            Self::SetKeys { .. } => "ansible.posix.authorized_key",

            Self::EnableSudo { .. } | Self::UseRootPWForSudo { .. } => "ansible.builtin.lineinfile",
        }
    }

    /// Returns a map of arguments that configure the module for this task.
    pub fn module_map(&self) -> HashMap<String, String> {
        match self {
            Self::DeleteFile { path } => HashMap::from([
                ("path".to_string(), path.clone()),
                ("state".to_string(), "absent".to_string()),
            ]),
            Self::ReadFile { path, .. } => HashMap::from([(
                "cmd".to_string(),
                format!("[ ! -f {} ] || cat {}", path, path),
            )]),
            Self::ChownDir { path, owner, group } => {
                let mut outmap = HashMap::from([
                    ("path".to_string(), path.clone()),
                    ("owner".to_string(), owner.clone()),
                    ("recurse".to_string(), "yes".to_string()),
                ]);
                if group.is_some() {
                    outmap.insert("group".to_string(), group.as_ref().unwrap().to_string());
                } else {
                    outmap.insert("group".to_string(), owner.clone());
                }
                outmap
            }
            Self::CreateUser { name } => HashMap::from([
                ("name".to_string(), name.clone()),
                ("state".to_string(), "present".to_string()),
            ]),
            Self::RecordJumpUser { names } => HashMap::from([
                ("dest".to_string(), JUMP_USER_FILE.to_string()),
                ("state".to_string(), "present".to_string()),
                ("force".to_string(), "true".to_string()),
                ("content".to_string(), names.join("\n")),
            ]),
            Self::PruneUsers { .. } => HashMap::from([
                ("name".to_string(), "{{ item }}".to_string()),
                ("state".to_string(), "absent".to_string()),
            ]),
            Self::AddUsers { .. } | Self::AddSudoers { .. } => HashMap::from([
                ("name".to_string(), "{{ item }}".to_string()),
                ("state".to_string(), "present".to_string()),
            ]),
            Self::AddSuperUsers { .. } => HashMap::from([
                ("name".to_string(), "{{ item }}".to_string()),
                ("state".to_string(), "present".to_string()),
                ("append".to_string(), "true".to_string()),
                ("groups".to_string(), "root".to_string()),
                ("non_unique".to_string(), "true".to_string()),
                ("password_lock".to_string(), "true".to_string()),
                ("uid".to_string(), "0".to_string()),
            ]),
            Self::SetKeys { user, keys } => HashMap::from([
                ("key".to_string(), keys.clone()),
                ("comment".to_string(), format!("jump_user: {user}")),
                ("user".to_string(), user.clone()),
                ("manage_dir".to_string(), "true".to_string()),
                ("exclusive".to_string(), "true".to_string()),
            ]),
            Self::EnableSudo { name } => HashMap::from([
                (
                    "path".to_string(),
                    "/etc/sudoers.d/ansible-sshman".to_string(),
                ),
                ("state".to_string(), "present".to_string()),
                ("create".to_string(), "yes".to_string()),
                ("line".to_string(), format!("{} ALL = (ALL) ALL", name)),
            ]),
            Self::UseRootPWForSudo { name } => HashMap::from([
                (
                    "path".to_string(),
                    "/etc/sudoers.d/ansible-sshman".to_string(),
                ),
                ("state".to_string(), "present".to_string()),
                ("create".to_string(), "yes".to_string()),
                ("line".to_string(), format!("Defaults:{} rootpw", name)),
            ]),
        }
    }

    //     /// Returns a vector of tasks to be used when setting up a new user.
    //     pub fn user_tasks(user: &SSHUser) -> Vec<Self> {
    //         let mut tasks = vec![
    //             Self::CreateUser {
    //                 name: user.name.clone(),
    //             },
    //             Self::RecordJumpUser {
    //                 names: user.name.clone(),
    //             },
    //         ];
    //         for pubkey in &user.pubkeys {
    //             tasks.push(Self::AuthorizeKey {
    //                 name: user.name.clone(),
    //                 pubkey: pubkey.to_owned(),
    //             })
    //         }
    //         tasks.push(Self::ChownDir {
    //             path: format!("/home/{}/", user.name.clone()),
    //             owner: user.name.clone(),
    //             group: None,
    //         });
    //         if user.sudoer == true {
    //             tasks.push(Self::EnableSudo {
    //                 name: user.name.clone(),
    //             });
    //             tasks.push(Self::UseRootPWForSudo {
    //                 name: user.name.clone(),
    //             });
    //         }
    //         return tasks;
    //     }
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

            Self::PruneUsers {
                found_var,
                allowed_var,
            } => {
                task.serialize_entry(
                    "loop",
                    &format!("{{{{ {found_var}.stdout_lines | difference({allowed_var}) }}}}"),
                )?;
                task.serialize_entry("ignore_errors", &true)?;
            }

            Self::AddUsers {
                found_var,
                allowed_var,
            }
            | Self::AddSudoers {
                found_var,
                allowed_var,
            }
            | Self::AddSuperUsers {
                found_var,
                allowed_var,
            } => {
                task.serialize_entry(
                    "loop",
                    &format!("{{{{ {allowed_var} | difference({found_var}.stdout_lines) }}}}"),
                )?;
                task.serialize_entry("ignore_errors", &true)?;
            }

            _ => {}
        }
        task.end()
    }
}
