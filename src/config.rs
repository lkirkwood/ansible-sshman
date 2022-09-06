use crate::{inventory::{SectionContainer, Inventory}, error::InvalidConfigError};

use std::{error::Error, collections::{HashMap, HashSet}};
use yaml_rust::{YamlLoader, Yaml};
use serde::ser::{Serialize, SerializeSeq};

#[derive(Debug, Hash, Eq, PartialEq)]
pub struct SSHUser {
    pub name: String,
    pub pubkey: String,
    pub access: String
}
impl Serialize for SSHUser {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
        where
            S: serde::Serializer {
        let mut seq = serializer.serialize_seq(Some(4))?;

        seq.serialize_element(&HashMap::from([
            ("name", format!("Create user {}", self.name))
        ]));
        return seq.end()
    }
}

pub struct SSHConfig {
    pub users: Vec<SSHUser>,
    pub inv: Inventory
}

impl SSHConfig {
    pub fn get_hosts(&self, user: &SSHUser) -> Vec<&str> {
        let sec = self.inv.get_by_path(&user.access).unwrap();
        return sec.descended_hosts()
    }

    /// Maps section paths to the set of users that can access.
    fn get_section_users(&self) -> HashMap<&str, HashSet<&SSHUser>> {
        let mut path_users = HashMap::new();
        for user in &self.users {
            let path = user.access.as_str();
            if !path_users.contains_key(path) {
                path_users.insert(path, HashSet::new());
            }
            path_users.get_mut(path).unwrap().insert(user);
        }
        todo!("Implement paths inheriting their parent's users.");
        return path_users
    }

    /// Returns an ansible playbook that ratifies the settings in this sshconf.
    pub fn playbook(&self) -> Result<String, Box<dyn Error>> {
        let mut outstr = String::new();
        for (path, users) in self.get_section_users() {
            for user in users {
                outstr.push_str(&serde_yaml::to_string(user)?);
            }
        }
        return Ok(outstr)
    }
}

// Parsing

#[derive(Debug)]
pub struct SSHConfigParser {}

impl SSHConfigParser {
    pub fn conf_from_string(
        inv: Inventory, content: String
    ) -> Result<SSHConfig, Box<dyn Error>> {
        let yaml = &YamlLoader::load_from_str(&content)?[0];
        let conf = match yaml {
            Yaml::Array(arr) => arr,
            _ => return Err(Box::new(InvalidConfigError::from_str(
                "Config file must be a array.")))
        };

        let mut users = Vec::new();
        for conf_entry in conf {
            users.push(
                Self::user_from_yaml(&inv, conf_entry)?);
        }

        return Ok(SSHConfig { users, inv })
    }

    /// Creates a user from a yaml obj if possible.
    fn user_from_yaml(
        inv: &Inventory, yaml: &Yaml
    ) -> Result<SSHUser, InvalidConfigError> {
        let user_def = match yaml {
            Yaml::Hash(map) => map,
            _ => return Err(InvalidConfigError::from_str(
                "All user definitions must be maps."))
        };

        let mut name = None;
        let mut pubkey = None;
        let mut access = None;
        for (yaml_key, yaml_val) in user_def {
            let (key, val) = match (yaml_key, yaml_val) {
                (Yaml::String(skey), Yaml::String(sval)) => (skey, sval),
                _ => return Err(InvalidConfigError::from_str(
                    "Keys and values of user definitions must be strings." ))
            };

            match key.as_str() {
                "name" => { name = Some(val); },
                "public_key" => { pubkey = Some(val); },
                "access" => { access = Some(val); },
                _ => return Err(InvalidConfigError {
                    message: format!("Unknown user definition key: {}", key)})
            }
        }

        if name.is_some() & pubkey.is_some() & access.is_some() {
            match inv.get_by_path(access.unwrap()) {
                None => return Err(InvalidConfigError {message: format!(
                        "Failed to find section in inventory with path: {}", access.unwrap())}),
                Some(_) => {
                    return Ok(SSHUser {
                        name: name.unwrap().to_string(),
                        pubkey: pubkey.unwrap().to_string(),
                        access: access.unwrap().to_string()
                    })
                }
            }
            
        } else {
            return Err(InvalidConfigError::from_str(
                "All user definitions must have the keys: name, public_key, access"))
        }
    }
}