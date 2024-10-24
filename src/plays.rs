use std::collections::HashMap;

use serde_yaml::Value;

use crate::{
    config::{Role, SSHUser},
    model::{AnsibleModule, AnsiblePlay, AnsibleTask},
};

impl AnsiblePlay {
    /// Returns a play which will create necessary groups on all hosts.
    pub fn create_groups() -> Self {
        Self {
            name: "Create groups.".to_string(),
            hosts: "all".to_string(),
            gather_facts: false,
            r#become: true,
            tasks: vec![
                AnsibleTask {
                    name: "Create sudoer group.",
                    module: AnsibleModule::groups(HashMap::from([(
                        "name",
                        Role::Sudoer.group().to_string(),
                    )])),
                    params: HashMap::new(),
                },
                AnsibleTask {
                    name: "Set sudo permissions for sudoers.",
                    module: AnsibleModule::sudo_file(Role::Sudoer),
                    params: HashMap::new(),
                },
                AnsibleTask {
                    name: "Create nopass group.",
                    module: AnsibleModule::groups(HashMap::from([(
                        "name",
                        Role::Nopass.group().to_string(),
                    )])),
                    params: HashMap::new(),
                },
                AnsibleTask {
                    name: "Set sudo permissions for nopasss.",
                    module: AnsibleModule::sudo_file(Role::Nopass),
                    params: HashMap::new(),
                },
            ],
        }
    }

    /// Creates the user if they do not already exist, and sets their group.
    pub fn create_user(user: &SSHUser) -> Vec<Self> {
        user.access
            .iter()
            .map(|(hosts, role)| Self {
                name: format!("Create accounts for {}.", user.name),
                hosts: hosts.to_string(),
                gather_facts: false,
                r#become: true,
                tasks: match role {
                    Role::SuperUser => vec![
                        AnsibleTask {
                            name: "Create root alias.",
                            module: AnsibleModule::users(HashMap::from([
                                ("name", user.name.to_owned()),
                                ("group", role.group().to_string()),
                                ("groups", role.group().to_string()),
                                ("non_unique", "true".to_string()),
                                ("uid", "0".to_string()),
                            ])),
                            params: HashMap::new(),
                        },
                        AnsibleTask {
                            name: "Remove root alias password.",
                            module: AnsibleModule::users(HashMap::from([
                                ("name", user.name.to_owned()),
                                ("password", "*".to_string()),
                            ])),
                            params: HashMap::new(),
                        },
                    ],
                    Role::Sudoer | Role::Nopass => vec![
                        AnsibleTask {
                            name: "Create sudoer account.",
                            module: AnsibleModule::users(HashMap::from([
                                ("name", user.name.to_owned()),
                                ("group", role.group().to_string()),
                                ("groups", role.group().to_string()),
                            ])),
                            params: HashMap::new(),
                        },
                        AnsibleTask {
                            name: "Remove sudoer account password.",
                            module: AnsibleModule::users(HashMap::from([
                                ("name", user.name.to_owned()),
                                ("password", "*".to_string()),
                            ])),
                            params: HashMap::new(),
                        },
                    ],
                    Role::Blocked => vec![],
                },
            })
            .collect()
    }

    /// Authorizes keys for a user.
    /// For blocked users this play can fail silently if they do not already have an account.
    pub fn authorize_keys(user: &SSHUser) -> Vec<Self> {
        user.access
            .iter()
            .map(|(hosts, role)| Self {
                name: format!("Authorize keys for {}.", &user.name),
                hosts: hosts.to_string(),
                r#become: true,
                gather_facts: false,
                tasks: vec![AnsibleTask {
                    name: "Authorize public key.",
                    module: AnsibleModule::keys(HashMap::from([
                        ("user", user.name.to_owned()),
                        ("key", user.pubkeys.join("\n")),
                        ("exclusive", "true".to_string()),
                        (
                            "state",
                            if *role == Role::Blocked {
                                "absent".to_string()
                            } else {
                                "present".to_string()
                            },
                        ),
                    ])),
                    params: if *role == Role::Blocked {
                        HashMap::from([("ignore_errors", Value::Bool(true))])
                    } else {
                        HashMap::new()
                    },
                }],
            })
            .collect()
    }
}
