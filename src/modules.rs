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
                    ("dest", format!("/etc/sudoers.d/{group}").into()),
                    ("mode", "440".into()),
                    ("validate", "visudo -cf %s".into()),
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
                    ("dest", format!("/etc/sudoers.d/{group}").into()),
                    ("mode", "440".into()),
                    ("validate", "visudo -cf %s".into()),
                ]),
            },
            other => panic!("Creating sudo file for role {other}"),
        }
    }

    /// Set some facts.
    pub fn set_facts(facts: HashMap<&'a str, Value>) -> Self {
        Self {
            name: "ansible.builtin.set_fact",
            params: facts,
        }
    }

    /// Use the getent utility and register the result as facts.
    pub fn getent(params: HashMap<&'a str, Value>) -> Self {
        Self {
            name: "ansible.builtin.getent",
            params,
        }
    }

    /// Slurps a file from a remote node.
    pub fn slurp(path: String) -> Self {
        Self {
            name: "ansible.builtin.slurp",
            params: HashMap::from([("src", Value::String(path))]),
        }
    }

    pub fn debug(msg: &str) -> Self {
        Self {
            name: "ansible.builtin.debug",
            params: HashMap::from([("msg", msg.into())]),
        }
    }
}
