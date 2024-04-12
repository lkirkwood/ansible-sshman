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
                    name: "Create regular user group.",
                    module: AnsibleModule::groups(HashMap::from([(
                        "name",
                        Role::User.group().to_string(),
                    )])),
                    params: HashMap::new(),
                },
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
                    module: AnsibleModule::sudo_file(
                        format!("%{}   ALL=ALL", Role::Sudoer.group()),
                        Role::Sudoer.group(),
                    ),
                    params: HashMap::new(),
                },
            ],
        }
    }

    /// Creates the user if they do not already exist, and sets their group.
    pub fn create_user(user: &SSHUser) -> Self {
        Self {
            name: format!("Create accounts for {}.", user.name),
            hosts: user.access.to_owned(),
            gather_facts: false,
            r#become: true,
            tasks: vec![AnsibleTask {
                name: "Create account.",
                module: AnsibleModule::users(if Role::SuperUser == user.role {
                    HashMap::from([
                        ("name", user.name.to_owned()),
                        ("group", user.role.group().to_string()),
                        ("non_unique", "true".to_string()),
                        ("uid", "0".to_string()),
                    ])
                } else {
                    HashMap::from([
                        ("name", user.name.to_owned()),
                        ("group", user.role.group().to_string()),
                    ])
                }),
                params: HashMap::new(),
            }],
        }
    }

    /// Authorizes keys for a user.
    /// For blocked users this play can fail silently if they do not already have an account.
    pub fn authorize_keys(usr: &SSHUser) -> Self {
        Self {
            name: format!("Authorize keys for {}.", &usr.name),
            hosts: usr.access.to_owned(),
            r#become: true,
            gather_facts: false,
            tasks: vec![AnsibleTask {
                name: "Authorize public key.",
                module: AnsibleModule::keys(HashMap::from([
                    ("user", usr.name.to_owned()),
                    ("key", usr.pubkeys.join("\n")),
                    ("exclusive", "true".to_string()),
                    (
                        "state",
                        if let Role::Blocked = usr.role {
                            "absent".to_string()
                        } else {
                            "present".to_string()
                        },
                    ),
                ])),
                params: if usr.role == Role::Blocked {
                    HashMap::from([("ignore_errors", Value::Bool(true))])
                } else {
                    HashMap::new()
                },
            }],
        }
    }
}