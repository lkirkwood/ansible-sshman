use std::collections::HashMap;

use serde_yaml::Value;

use crate::{
    config::{Role, SSHConfig, SSHUser},
    model::{AnsibleModule, AnsiblePlay, AnsibleTask},
};

impl<'a> AnsiblePlay<'a> {
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

    pub fn set_desired_pubkey_facts(conf: &'a SSHConfig) -> Vec<Self> {
        let mut plays = vec![];
        for user in &conf.users {
            for group in user.access.keys() {
                plays.push(AnsiblePlay {
                    name: format!(
                        "Populate desired pubkey facts for {} on hosts in group {group}",
                        user.name
                    ),
                    hosts: group.to_string(),
                    gather_facts: false,
                    r#become: false,
                    tasks: vec![AnsibleTask {
                        name: "Populate desired pubkey facts",
                        module: AnsibleModule::set_facts(HashMap::from([(
                            "desired_pubkeys",
                            format!(
                                "{{{{ desired_pubkeys | default({{}}) | combine({{\"{}\": [\"{}\"]}}) }}}}",
                                user.name,
                                user.pubkeys.join("\", \"")
                            )
                            .into(),
                        )])),
                        params: HashMap::new(),
                    }],
                })
            }
        }

        plays
    }

    pub fn set_actual_pubkey_facts() -> Vec<Self> {
        vec![AnsiblePlay {
            name: "Populate actual pubkey facts for all hosts".to_string(),
            hosts: "all".to_string(),
            gather_facts: false,
            r#become: false,
            tasks: vec![
                AnsibleTask {
                    name: "Read contents of passwd db",
                    module: AnsibleModule::getent(HashMap::from([("database", "passwd".into())])),
                    params: HashMap::new(),
                }, // Read pubkey file for each user
                AnsibleTask {
                    name: "Append username to passwd items",
                    module: AnsibleModule::set_facts(HashMap::from([(
                        "getent_passwd",
                        "{{ getent_passwd | combine({item.key: item.value + [item.key]}) }}".into(),
                    )])),
                    params: HashMap::from([
                        ("loop", "{{ getent_passwd | dict2items }}".into()),
                        ("delegate_to", "localhost".into()),
                        ("run_once", true.into()),
                    ]),
                },
                AnsibleTask {
                    name: "Read authorized_keys for each user",
                    module: AnsibleModule::slurp("{{ item[4] }}/.ssh/authorized_keys".to_string()),
                    params: HashMap::from([
                        ("loop", "{{ getent_passwd.values() }}".into()),
                        ("register", "pubkey_files".into()),
                        ("ignore_errors", true.into()),
                        ("become", true.into()),
                    ]),
                },
                AnsibleTask {
                    name: "Populate actual pubkey facts",
                    module: AnsibleModule::set_facts(HashMap::from([(
                        "actual_pubkeys",
                        "{{ actual_pubkeys | default({}) | combine({item.item[-1]: item.content | trim | b64decode | split('\n') | reject('equalto', '') }) }}".into(),
                    )])),
                    params: HashMap::from([
                        ("loop", "{{ pubkey_files.results }}".into()),
                        ("when", "item.failed != True".into()),
                    ]),
                },
            ],
        }]
    }

    /// Validates the set of users on each host with authorized public keys against the config.
    pub fn validate(conf: &'a SSHConfig) -> Vec<Self> {
        let mut plays = vec![];
        plays.extend(Self::set_desired_pubkey_facts(conf));
        plays.extend(Self::set_actual_pubkey_facts());
        plays.extend(vec![Self {
            name: "Validate authorized keys".to_string(),
            hosts: "all".to_string(),
            gather_facts: false,
            r#become: false,
            tasks: vec![
                AnsibleTask {
                    name: "Compute differences in desired and actual pubkey lists",
                    module: AnsibleModule::set_facts(HashMap::from([(
                        "_pubkey_diff",
                        "{{ _pubkey_diff | default({}) | combine({item.key: item.value | reject('in', desired_pubkeys[item.key] | default([]))}) }}"
                            .into(),
                    )])),
                    params: HashMap::from([("loop", "{{ actual_pubkeys | dict2items }}".into())]),
                },
                AnsibleTask {
                    name: "Filter pubkey diff list",
                    module: AnsibleModule::set_facts(HashMap::from([(
                        "pubkey_diff",
                        "{{ pubkey_diff | default({}) | combine({item.key: item.value}) }}"
                            .into(),
                    )])),
                    params: HashMap::from([
                        ("loop", "{{ _pubkey_diff | dict2items }}".into()),
                        ("when", "item.value | length > 0".into())
                    ]),
                },
                 AnsibleTask {
                       name: "Print extra users",
                       module: AnsibleModule::debug("{{ actual_pubkeys[item.key] }}"),
                       params: HashMap::from([
                           ("loop", "{{ pubkey_diff | default({}) | dict2items }}".into()),
                           ("failed_when", "pubkey_diff | default({}) | length > 0".into()),
                       ]),
                   },
            ],
        }]);

        plays
    }
}
