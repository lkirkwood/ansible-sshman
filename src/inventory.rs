use serde::{Deserialize, Serialize};

use std::collections::{HashMap, HashSet};

#[derive(Debug, PartialEq, Eq, Deserialize, Serialize)]
#[serde(transparent)]
pub struct Inventory {
    pub groups: HashMap<String, Group>,
}

impl Inventory {
    pub fn get_pattern_hosts(&self, pattern: &str) -> HashSet<&str> {
        let mut hosts = HashSet::new();

        let names = pattern.split([':', ',']);
        for name in names {
            let raw_name = name.trim_start_matches(['&', '!']);

            if name.starts_with('&') {
                if let Some(group) = self.groups.get(raw_name) {
                    hosts = hosts.intersection(&group.hosts()).copied().collect()
                }
            } else if name.starts_with('!') {
                if let Some(group) = self.groups.get(raw_name) {
                    hosts = hosts.difference(&group.hosts()).copied().collect()
                }
            } else if let Some(group) = self.groups.get(name) {
                hosts.extend(group.hosts())
            }
        }

        hosts
    }
}

#[derive(Debug, PartialEq, Eq, Deserialize, Serialize)]
pub struct Group {
    /// Hostnames in the group.
    #[serde(default)]
    pub hosts: HashMap<String, serde_yaml::Value>,

    /// Groups nested under this group.
    #[serde(default)]
    pub children: HashMap<String, Group>,
}

impl Group {
    /// Returns all hosts in this group.
    /// Includes hosts in subgroups.
    pub fn hosts(&self) -> HashSet<&str> {
        let mut outset = self
            .hosts
            .keys()
            .map(|h| h.as_str())
            .collect::<HashSet<&str>>();

        for group in self.children.values() {
            outset.extend(group.hosts());
        }

        outset
    }
}
