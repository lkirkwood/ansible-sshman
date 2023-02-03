use crate::error::UndefinedGroupError;

use std::collections::{HashMap, HashSet};

#[derive(Debug)]
pub struct Inventory {
    /// Hostnames defined in the inventory outside of a group.
    pub hosts: Vec<String>,
    /// Internal group object for wildcard group.
    group: Group,
}

impl Inventory {
    pub fn get_group(&self, name: &str) -> Option<&Group> {
        if name == "*" {
            return Some(&self.group);
        } else {
            for group in self.group.descendants() {
                if group.name == name {
                    return Some(group);
                }
            }
        }
        return None;
    }

    /// Gets the hosts that are targeted by an access path.
    pub fn get_path_hosts(&self, path: &str) -> HashSet<&str> {
        let mut path_hosts = HashSet::new();
        for cmp in path.split([':', ',']) {
            let cmp_hosts = HashSet::from_iter(
                self.get_group(cmp.trim_start_matches(['&', '!']))
                    .unwrap()
                    .descended_hosts(),
            );
            if cmp.starts_with('&') {
                path_hosts = path_hosts.intersection(&cmp_hosts).map(|s| *s).collect();
            } else if cmp.starts_with('!') {
                path_hosts = path_hosts.difference(&cmp_hosts).map(|s| *s).collect();
            } else {
                path_hosts.extend(cmp_hosts);
            }
        }
        return path_hosts;
    }
}

#[derive(Debug)]
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
    children: HashMap<String, Group>,
}

impl Group {
    pub fn new(path: String) -> Group {
        let depth = path.matches(':').count();

        let name: String;
        if depth > 0 {
            name = path.rsplit_once(':').unwrap().1.to_string();
        } else {
            name = path.clone();
        }

        return Group {
            name,
            path,
            hosts: Vec::new(),
            depth,
            children: HashMap::new(),
        };
    }
}

pub trait GroupContainer<'a> {
    fn path(&self) -> &str;

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
        return children;
    }

    /// Returns all hosts defined directly within this group.
    fn child_hosts(&'a self) -> Vec<&'a str>;

    /// Returns all hosts descended from this one.
    fn descended_hosts(&'a self) -> Vec<&'a str> {
        let mut hosts = self.child_hosts().clone();
        for child in self.children() {
            hosts.extend(child.descended_hosts());
        }
        return hosts;
    }

    // /// Gets a group from its path relative to this object.
    // /// Wildcard ("*") indicates all groups.
    // fn get_by_path(&'a self, path: &str) -> Option<&'a dyn (GroupContainer)>;
}

impl<'a> GroupContainer<'a> for Group {
    fn path(&self) -> &str {
        return &self.path;
    }

    fn children(&'a self) -> Vec<&'a Group> {
        return self.children.values().collect();
    }

    fn child_hosts(&'a self) -> Vec<&'a str> {
        return self.hosts.iter().map(|s| &**s).collect();
    }

    // fn get_by_path(&'a self, path: &str) -> Option<&'a dyn (GroupContainer)> {
    //     if path == "*" {
    //         return Some(self);
    //     } else {
    //         let mut group = self;
    //         for name in path.split(':') {
    //             if group.children.contains_key(name) {
    //                 group = group.children.get(name).unwrap();
    //             } else {
    //                 return None;
    //             }
    //         }
    //         return Some(group);
    //     }
    // }
}

impl<'a> GroupContainer<'a> for Inventory {
    fn path(&self) -> &str {
        return "*";
    }

    fn children(&'a self) -> Vec<&'a Group> {
        return self.group.children();
    }

    fn child_hosts(&'a self) -> Vec<&'a str> {
        return self.hosts.iter().map(|s| &**s).collect();
    }

    // fn get_by_path(&'a self, path: &str) -> Option<&'a dyn (GroupContainer)> {
    //     if path == "*" {
    //         return Some(&self.group);
    //     } else {
    //         let names: Vec<&str> = path.split(':').collect();
    //         let first = *names.first().unwrap();
    //         if self.group.children().contains_key(first) {
    //             let mut group = self.groups.get(first).unwrap();
    //             for name in &names[1..] {
    //                 if group.children.contains_key(*name) {
    //                     group = group.children.get(*name).unwrap();
    //                 } else {
    //                     return None;
    //                 }
    //             }
    //             return Some(group);
    //         } else {
    //             return None;
    //         }
    //     }
    // }
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
        let name: String;
        if path.contains(':') {
            name = path.rsplit_once(':').unwrap().1.to_string();
        } else {
            name = path.clone();
        }
        GroupOutline {
            name,
            path,
            children: Vec::new(),
            hosts: Vec::new(),
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
            match self.outlines.get(&path) {
                None => return Err(UndefinedGroupError { name: path }),
                Some(child_outline) => {
                    let child = self.resolve_outline(child_outline)?;
                    group.children.insert(child_name.to_string(), child);
                }
            }
        }

        group.hosts = outline.hosts.clone();
        return Ok(group);
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
        let mut wildcard = Group::new("*".to_string());
        wildcard.children = groups;

        return Ok(Inventory {
            hosts: self.hosts.clone(),
            group: wildcard,
        });
    }

    /// Stores the data parsed from an inventory file.
    fn parse(&mut self) {
        let mut line_buf: &mut Vec<String> = &mut self.hosts;
        let mut last_path = "".to_string();
        for line in self.content.lines() {
            if line.starts_with("#") | line.trim().is_empty() {
                continue;
            } else if line.starts_with("[") {
                let mut name = line.trim()[1..line.len() - 1].to_string();
                if self.children_def == true {
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

    /// Returns a new inventory from a string.
    pub fn inv_from_string(content: String) -> Result<Inventory, UndefinedGroupError> {
        let mut parser = InventoryParser {
            content,
            outlines: HashMap::new(),
            hosts: Vec::new(),
            children: HashMap::new(),
            children_def: false,
        };
        parser.parse();
        return parser.to_inv();
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
        return None;
    }
}
