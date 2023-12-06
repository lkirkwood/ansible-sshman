use crate::error::UndefinedGroupError;

use std::collections::{HashMap, HashSet};

#[derive(Debug, Eq, PartialEq)]
pub struct Inventory {
    /// Root group.
    root: Group,
}

impl Inventory {
    pub fn get_group(&self, name: &str) -> Option<&Group> {
        if name == "all" {
            return Some(&self.root);
        } else {
            for group in self.root.descendants() {
                if group.name == name {
                    return Some(group);
                }
            }
        }
        None
    }

    /// Gets the hosts that are targeted by an access path.
    pub fn get_path_hosts(&self, path: &str) -> Option<HashSet<&str>> {
        let mut path_hosts = HashSet::new();
        for cmp in path.split([':', ',']) {
            let cmp_hosts = HashSet::from_iter(
                self.get_group(cmp.trim_start_matches(['&', '!']))?
                    .descended_hosts(),
            );
            if cmp.starts_with('&') {
                path_hosts = path_hosts.intersection(&cmp_hosts).copied().collect();
            } else if cmp.starts_with('!') {
                path_hosts = path_hosts.difference(&cmp_hosts).copied().collect();
            } else {
                path_hosts.extend(cmp_hosts);
            }
        }
        Some(path_hosts)
    }
}

#[derive(Debug, PartialEq, Eq)]
pub struct Group {
    /// Name of the group.
    pub name: String,
    /// Full path of the group.
    pub path: String,
    /// Hostnames in the group.
    pub hosts: Vec<String>,
    /// Nesting depth of this group (number of parent groups).
    pub depth: usize,
    /// Groups in this group.
    pub children: HashMap<String, Group>,
}

impl Group {
    /// Constructs a new group from its path
    /// e.g. group_1:group_1_2:group_1_2_1
    pub fn new(path: String) -> Group {
        let depth = path.matches(':').count();

        let name = if depth > 0 {
            path.rsplit_once(':').unwrap().1.to_string()
        } else {
            path.clone()
        };

        Group {
            name,
            path,
            hosts: Vec::new(),
            depth,
            children: HashMap::new(),
        }
    }
}

pub trait GroupContainer<'a> {
    /// Returns the path of this group
    fn path(&self) -> &str;

    /// Returns true if this group container contains the other group container.
    fn contains(&self, container: &(dyn GroupContainer)) -> bool {
        return self.path().contains(container.path());
    }
    /// Returns all the groups that exist directly within this one.
    fn children(&'a self) -> Vec<&'a Group>;

    /// Returns all the groups that are descended from this one.
    fn descendants(&'a self) -> Vec<&'a Group> {
        let mut children = self.children().clone();
        for child in self.children() {
            children.extend(child.descendants());
        }
        children
    }

    /// Returns all hosts defined directly within this group.
    fn child_hosts(&'a self) -> Vec<&'a str>;

    /// Returns all hosts descended from this one.
    fn descended_hosts(&'a self) -> Vec<&'a str> {
        let mut hosts = self.child_hosts().clone();
        for child in self.children() {
            hosts.extend(child.descended_hosts());
        }
        hosts
    }
}

impl<'a> GroupContainer<'a> for Group {
    fn path(&self) -> &str {
        &self.path
    }

    fn children(&'a self) -> Vec<&'a Group> {
        return self.children.values().collect();
    }

    fn child_hosts(&'a self) -> Vec<&'a str> {
        return self.hosts.iter().map(|s| &**s).collect();
    }
}

impl<'a> GroupContainer<'a> for Inventory {
    fn path(&self) -> &str {
        "all"
    }

    fn children(&'a self) -> Vec<&'a Group> {
        return self.root.children();
    }

    fn child_hosts(&'a self) -> Vec<&'a str> {
        return self.root.hosts.iter().map(|s| &**s).collect();
    }
}

// Parsing

#[derive(Debug, Hash, PartialEq, Eq)]
struct GroupOutline {
    pub name: String,
    pub path: String,
    pub children: Vec<String>,
    pub hosts: Vec<String>,
}

impl GroupOutline {
    pub fn new(path: String) -> GroupOutline {
        let name = match path.rsplit_once(':') {
            Some(split) => split.1.to_owned(),
            None => path.to_owned(),
        };

        GroupOutline {
            name,
            path,
            children: vec![],
            hosts: vec![],
        }
    }
}

enum InvGroupDefType {
    Hosts,
    Children,
}

pub struct InventoryParser {
    /// Content to parse an inventory from.
    content: String,
    /// Group outlines, mapped by name.
    outlines: HashMap<String, GroupOutline>,
    /// Hosts defined outside of a group.
    hosts: Vec<String>,

    /// Maps group path to a list of its child group names.
    children: HashMap<String, Vec<String>>,
    /// Whether the parser is currently in a group children definition.
    children_def: bool,
}

impl InventoryParser {
    /// Recursively resolves this outline and its children to groups.
    fn resolve_outline(&self, outline: &GroupOutline) -> Result<Group, UndefinedGroupError> {
        let mut group = Group::new(outline.path.to_string());
        for child_name in &outline.children {
            let path = format!("{}:{}", outline.path, child_name);
            let child = match self.outlines.get(&path) {
                None => Group::new(path),
                Some(child_outline) => self.resolve_outline(child_outline)?,
            };
            group.children.insert(child.name.clone(), child);
        }

        group.hosts = outline.hosts.clone();
        Ok(group)
    }

    /// Creates an inventory from stored data.
    fn to_inv(&self) -> Result<Inventory, UndefinedGroupError> {
        let mut groups = HashMap::new();
        for outline in self.outlines.values() {
            if !outline.path.contains(':') {
                let group = self.resolve_outline(outline)?;
                groups.insert(group.name.clone(), group);
            }
        }
        let mut root = Group::new("all".to_string());
        root.children = groups;
        root.hosts = self.hosts.clone();

        Ok(Inventory { root })
    }

    /// Stores the data parsed from an inventory file.
    fn parse(&mut self) {
        let mut line_buf = &mut self.hosts;
        let mut last_path = "".to_string();
        for line in self.content.lines() {
            if line.starts_with('#') | line.trim().is_empty() {
                continue;
            } else if line.starts_with('[') {
                let mut name = line.trim()[1..line.len() - 1].to_string();
                if self.children_def {
                    // leaving children def
                    self.children.insert(last_path, line_buf.clone());
                    self.children_def = false;
                }

                let def_type: InvGroupDefType;
                if name.ends_with(":children") {
                    name.truncate(name.len() - 9);
                    def_type = InvGroupDefType::Children;
                    self.children_def = true;
                } else {
                    def_type = InvGroupDefType::Hosts;
                }

                let path = match self.children.first_key_for(&name) {
                    None => name.clone(),
                    Some(s) => format!("{}:{}", s, name).to_string(),
                };

                let outline = match self.outlines.get_mut(&path) {
                    None => {
                        let _outline = GroupOutline::new(path.clone());
                        self.outlines.insert(_outline.path.clone(), _outline);
                        self.outlines.get_mut(&path).unwrap()
                    }
                    Some(_outline) => _outline,
                };

                line_buf = match def_type {
                    InvGroupDefType::Children => &mut outline.children,
                    InvGroupDefType::Hosts => &mut outline.hosts,
                };
                last_path = path;
            } else {
                line_buf.push(line.to_string());
            }
        }
    }

    /// Returns a new inventory from the contents of an ini file.
    pub fn inv_from_ini(content: String) -> Result<Inventory, UndefinedGroupError> {
        let mut parser = InventoryParser {
            content,
            outlines: HashMap::new(),
            hosts: Vec::new(),
            children: HashMap::new(),
            children_def: false,
        };
        parser.parse();
        parser.to_inv()
    }
}

// Helpers

trait Finder<T> {
    fn first_key_for(&self, val: &T) -> Option<&T>;
}

impl<T: Eq> Finder<T> for HashMap<T, Vec<T>> {
    fn first_key_for(&self, val: &T) -> Option<&T> {
        for (key, values) in self {
            if values.contains(val) {
                return Some(key);
            }
        }
        None
    }
}

#[cfg(test)]
mod tests {
    use std::fs;

    use super::*;

    #[test]
    fn test_inventory_parse() {
        let inv =
            InventoryParser::inv_from_ini(fs::read_to_string("test/test_inv").unwrap()).unwrap();
        assert_eq!(dummy_inv(), inv)
    }

    fn dummy_inv() -> Inventory {
        Inventory {
            root: Group {
                name: "all".to_string(),
                path: "all".to_string(),
                hosts: vec!["host-1".to_string(), "host-2".to_string()],
                depth: 0,
                children: HashMap::from([(
                    "group_1".to_string(),
                    Group {
                        name: "group_1".to_string(),
                        path: "group_1".to_string(),
                        hosts: vec!["host-1-1".to_string(), "host-1-2".to_string()],
                        depth: 0,
                        children: HashMap::from([
                            (
                                "group_1_1".to_string(),
                                Group {
                                    name: "group_1_1".to_string(),
                                    path: "group_1:group_1_1".to_string(),
                                    hosts: vec!["host-1-1-1".to_string(), "host-1-1-2".to_string()],
                                    depth: 1,
                                    children: HashMap::new(),
                                },
                            ),
                            (
                                "group_1_3".to_string(),
                                Group {
                                    name: "group_1_3".to_string(),
                                    path: "group_1:group_1_3".to_string(),
                                    hosts: vec![],
                                    depth: 1,
                                    children: HashMap::new(),
                                },
                            ),
                            (
                                "group_1_2".to_string(),
                                Group {
                                    name: "group_1_2".to_string(),
                                    path: "group_1:group_1_2".to_string(),
                                    hosts: vec!["host-1-2-1".to_string()],
                                    depth: 1,
                                    children: HashMap::from([(
                                        "group_1_2_1".to_string(),
                                        Group {
                                            name: "group_1_2_1".to_string(),
                                            path: "group_1:group_1_2:group_1_2_1".to_string(),
                                            hosts: vec![
                                                "host-1-2-1-1".to_string(),
                                                "host-1-2-1-2".to_string(),
                                                "host-1-2-1-3".to_string(),
                                            ],
                                            depth: 2,
                                            children: HashMap::new(),
                                        },
                                    )]),
                                },
                            ),
                        ]),
                    },
                )]),
            },
        }
    }
}
