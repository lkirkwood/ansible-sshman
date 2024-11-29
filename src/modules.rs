use std::collections::HashMap;

use serde_yaml::Value;

use crate::{config::Role, model::AnsibleModule};

impl<'a> AnsibleModule<'a> {
    /// Ansible module for manipulating groups.
    pub fn groups(params: HashMap<&'static str, String>) -> Self {
        Self {
            name: "ansible.builtin.group",
            params: params
                .into_iter()
                .map(|(k, v)| (k, Value::String(v)))
                .collect(),
        }
    }

    /// Ansible module for manipulating users.
    pub fn users(params: HashMap<&'static str, String>) -> Self {
        Self {
            name: "ansible.builtin.user",
            params: params
                .into_iter()
                .map(|(k, v)| (k, Value::String(v)))
                .collect(),
        }
    }

    /// Ansible module for authorizing keys.
    pub fn keys(params: HashMap<&'static str, String>) -> Self {
        Self {
            name: "ansible.posix.authorized_key",
            params: params
                .into_iter()
                .map(|(k, v)| (k, Value::String(v)))
                .collect(),
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
                        Value::String(format!(
                            "{}\n{}\n",
                            format!("%{group} ALL=(ALL) NOPASSWD: ALL"),
                            format!("Defaults:%{group} !requiretty"),
                        )),
                    ),
                    ("dest", Value::String(format!("/etc/sudoers.d/{group}"))),
                    ("mode", Value::String("440".to_string())),
                    ("validate", Value::String("visudo -cf %s".to_string())),
                ]),
            },
            Role::Sudoer => Self {
                name: "ansible.builtin.copy",
                params: HashMap::from([
                    (
                        "content",
                        Value::String(format!(
                            "{}\n{}\n",
                            format!("%{group} ALL=(ALL) ALL"),
                            format!("Defaults:%{group} rootpw"),
                        )),
                    ),
                    ("dest", Value::String(format!("/etc/sudoers.d/{group}"))),
                    ("mode", Value::String("440".to_string())),
                    ("validate", Value::String("visudo -cf %s".to_string())),
                ]),
            },
            other => panic!("Creating sudo file for role {other}"),
        }
    }
}
