use std::collections::HashMap;

use serde::ser::SerializeMap;
use serde::Serialize;
use serde_yaml::Value;

/// Models an ansible play.
#[derive(Debug, Serialize)]
pub struct AnsiblePlay<'a> {
    /// Name of the play.
    pub name: String,
    /// Host pattern this play targets.
    pub hosts: String,
    /// Whether to gather facts before this play.
    pub gather_facts: bool,
    /// Whether to execute the whole play as root.
    pub r#become: bool,
    /// The tasks in this play.
    pub tasks: Vec<AnsibleTask<'a>>,
}

#[derive(Debug)]
/// A single task in an AnsiblePlay.
pub struct AnsibleTask<'a> {
    pub name: &'static str,
    pub module: AnsibleModule<'a>,
    pub params: HashMap<&'static str, Value>,
}

impl<'a> Serialize for AnsibleTask<'a> {
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
pub struct AnsibleModule<'a> {
    pub name: &'static str,
    pub params: HashMap<&'a str, Value>,
}
