use std::collections::HashMap;

use serde::ser::SerializeMap;
use serde::Serialize;
use serde_yaml::Value;

use crate::config::Role;

/// Models an ansible play.
#[derive(Debug, Serialize)]
pub struct AnsiblePlay {
    /// Name of the play.
    pub name: String,
    /// Host pattern this play targets.
    pub hosts: String,
    /// Whether to gather facts before this play.
    pub gather_facts: bool,
    /// Whether to execute the whole play as root.
    pub r#become: bool,
    /// The tasks in this play.
    pub tasks: Vec<AnsibleTask>,
}

#[derive(Debug)]
/// A single task in an AnsiblePlay.
pub struct AnsibleTask {
    pub name: &'static str,
    pub module: AnsibleModule,
    pub params: HashMap<&'static str, Value>,
}

impl Serialize for AnsibleTask {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        let mut map = serializer.serialize_map(Some(2 + self.params.len()))?;
        map.serialize_entry("name", &self.name)?;
        map.serialize_entry(&self.module.name, &self.module.params)?;

        for (key, value) in &self.params {
            map.serialize_entry(key, value)?;
        }

        map.end()
    }
}

#[derive(Debug)]
/// The module to call for an AnsibleTask.
pub struct AnsibleModule {
    pub name: &'static str,
    pub params: HashMap<&'static str, String>,
}

impl AnsibleModule {
    /// Ansible module for manipulating groups.
    pub fn groups(params: HashMap<&'static str, String>) -> Self {
        Self {
            name: "ansible.builtin.group",
            params,
        }
    }

    /// Ansible module for manipulating users.
    pub fn users(params: HashMap<&'static str, String>) -> Self {
        Self {
            name: "ansible.builtin.user",
            params,
        }
    }

    /// Ansible module for authorizing keys.
    pub fn keys(params: HashMap<&'static str, String>) -> Self {
        Self {
            name: "ansible.posix.authorized_key",
            params,
        }
    }

    /// Creates a sudo file for the group, allowing them to use sudo, with the rootpw flag set.
    /// Validates with visudo.
    pub fn sudo_file(role: Role) -> Self {
        let group = role.group();
        match role {
            Role::Nopass => Self {
                name: "ansible.builtin.copy",
                params: HashMap::from([
                    (
                        "content",
                        format!(
                            "{}\n{}\n",
                            format!("%{group} ALL=(ALL) NOPASSWD: ALL"),
                            format!("Defaults:%{group} !requiretty"),
                        ),
                    ),
                    ("dest", format!("/etc/sudoers.d/{group}")),
                    ("mode", "440".to_string()),
                    ("validate", "visudo -cf %s".to_string()),
                ]),
            },
            Role::Sudoer => Self {
                name: "ansible.builtin.copy",
                params: HashMap::from([
                    (
                        "content",
                        format!(
                            "{}\n{}\n",
                            format!("%{group} ALL=(ALL) ALL"),
                            format!("Defaults:%{group} rootpw"),
                        ),
                    ),
                    ("dest", format!("/etc/sudoers.d/{group}")),
                    ("mode", "440".to_string()),
                    ("validate", "visudo -cf %s".to_string()),
                ]),
            },
            other => panic!("Creating sudo file for role {other}"),
        }
    }
}
