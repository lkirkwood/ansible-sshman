use pretty_assertions::assert_eq;
use serde_yaml::Value;
use std::fs;

use crate::config::SSHConfig;

#[test]
fn test_playbook_output() {
    let conf: SSHConfig =
        serde_yaml::from_str(&fs::read_to_string("test/config.yml").unwrap()).unwrap();

    let actual_playbook = serde_yaml::to_value(conf.playbook()).unwrap();

    let expected_playbook: Value =
        serde_yaml::from_str(&fs::read_to_string("test/playbook.yml").unwrap()).unwrap();

    assert_eq!(actual_playbook, expected_playbook);
}
