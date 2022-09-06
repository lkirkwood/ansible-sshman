use std::{error::Error, collections::HashMap};

use crate::{inventory::{SectionContainer, Inventory}, error::InvalidConfigError};

use yaml_rust::{YamlLoader, Yaml, YamlEmitter, yaml::Hash};

pub struct SSHUser {
    pub name: String,
    pub pubkey: String,
    pub access: String
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