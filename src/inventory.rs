use crate::error::UndefinedSectionError;

use std::collections::{HashMap, HashSet};

#[derive(Debug)]
pub struct Inventory {
    /// Hostnames defined in the inventory outside of a section.
    hosts: Vec<String>,
    /// Top-level sections in the inventory.
    sections: HashMap<String, Section>,
}

#[derive(Debug)]
pub struct Section {
    /// Name of the section.
    pub name: String,
    /// Full path of the section.
    pub path: String,
    /// Hostnames in the section.
    pub hosts: Vec<String>,
    /// Sections in this section.
    children: HashMap<String, Section>,
}

impl Section {
    pub fn new(path: String) -> Section {
        let name: String;
        if path.contains(':') {
            name = path.rsplit_once(':').unwrap().1.to_string();
        } else {
            name = path.clone();
        }
        return Section {
            name,
            path,
            hosts: Vec::new(),
            children: HashMap::new(),
        };
    }
}

pub trait SectionContainer<'a> {
    /// Returns all the sections that exist directly within this one.
    fn children(&'a self) -> Vec<&'a Section>;

    /// Returns all the sections that are descended from this one.
    fn descendants(&'a self) -> Vec<&'a Section> {
        let mut children = self.children().clone();
        for child in self.children() {
            children.extend(child.descendants());
        }
        return children;
    }

    /// Returns all hosts defined directly within this section.
    fn child_hosts(&'a self) -> Vec<&'a str>;

    /// Returns all hosts descended from this one.
    fn descended_hosts(&'a self) -> Vec<&'a str> {
        let mut hosts = self.child_hosts().clone();
        for child in self.children() {
            hosts.extend(child.descended_hosts());
        }
        return hosts;
    }

    /// Gets a section from its path relative to this object.
    /// Wildcard ("*") indicates all sections.
    fn get_by_path(&'a self, path: &str) -> Option<&'a dyn (SectionContainer)>;
}

impl<'a> SectionContainer<'a> for Section {
    fn children(&'a self) -> Vec<&'a Section> {
        return self.children.values().collect();
    }

    fn child_hosts(&'a self) -> Vec<&'a str> {
        return self.hosts.iter().map(|s| &**s).collect();
    }

    fn get_by_path(&'a self, path: &str) -> Option<&'a dyn (SectionContainer)> {
        if path == "*" {
            return Some(self);
        } else {
            let mut section = self;
            for name in path.split(':') {
                if section.children.contains_key(name) {
                    section = section.children.get(name).unwrap();
                } else {
                    return None;
                }
            }
            return Some(section);
        }
    }
}

impl<'a> SectionContainer<'a> for Inventory {
    fn children(&'a self) -> Vec<&'a Section> {
        return self.sections.values().collect();
    }

    fn child_hosts(&'a self) -> Vec<&'a str> {
        return self.hosts.iter().map(|s| &**s).collect();
    }

    fn get_by_path(&'a self, path: &str) -> Option<&'a dyn (SectionContainer)> {
        if path == "*" {
            return Some(self);
        } else {
            let names: Vec<&str> = path.split(':').collect();
            let first = *names.first().unwrap();
            if self.sections.contains_key(first) {
                let mut section = self.sections.get(first).unwrap();
                for name in &names[1..] {
                    if section.children.contains_key(*name) {
                        section = section.children.get(*name).unwrap();
                    } else {
                        return None;
                    }
                }
                return Some(section);
            } else {
                return None;
            }
        }
    }
}

// Parsing

#[derive(Debug, Hash, PartialEq, Eq)]
struct SectionOutline {
    pub name: String,
    pub path: String,
    pub children: Vec<String>,
    pub hosts: Vec<String>,
}

impl SectionOutline {
    pub fn new(path: String) -> SectionOutline {
        let name: String;
        if path.contains(':') {
            name = path.rsplit_once(':').unwrap().1.to_string();
        } else {
            name = path.clone();
        }
        SectionOutline {
            name,
            path,
            children: Vec::new(),
            hosts: Vec::new(),
        }
    }
}

enum InvSectionDefType {
    Hosts,
    Children,
}

pub struct InventoryParser {
    /// Content to parse an inventory from.
    content: String,
    /// Section outlines, mapped by name.
    outlines: HashMap<String, SectionOutline>,
    /// Hosts defined outside of a section.
    hosts: Vec<String>,

    /// Maps section path to a list of its child section names.
    children: HashMap<String, Vec<String>>,
    /// Whether the parser is currently in a section children definition.
    children_def: bool,
}

impl InventoryParser {
    /// Recursively resolves this outline and its children to sections.
    fn resolve_outline(&self, outline: &SectionOutline) -> Result<Section, UndefinedSectionError> {
        let mut section = Section::new(outline.path.to_string());
        for child_name in &outline.children {
            let path = format!("{}:{}", outline.path, child_name);
            match self.outlines.get(&path) {
                None => return Err(UndefinedSectionError { name: path }),
                Some(child_outline) => {
                    let child = self.resolve_outline(child_outline)?;
                    section.children.insert(child_name.to_string(), child);
                }
            }
        }

        if outline.path == "org_all" {
            println!("outline children: {:?}", outline.children);
            println!("real children: {:?}", section.children);
        }
        section.hosts = outline.hosts.clone();
        return Ok(section);
    }

    /// Creates an inventory from stored data.
    fn to_inv(&self) -> Result<Inventory, UndefinedSectionError> {
        let mut sections = HashMap::new();
        for outline in self.outlines.values() {
            if !outline.path.contains(':') {
                let section = self.resolve_outline(outline)?;
                sections.insert(section.name.clone(), section);
            }
        }

        return Ok(Inventory {
            hosts: self.hosts.clone(),
            sections,
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
                    println!("{}: \n{:?}", last_path, line_buf.clone());
                    self.children.insert(last_path, line_buf.clone());
                    self.children_def = false;
                }

                let def_type: InvSectionDefType;
                if name.ends_with(":children") {
                    name.truncate(name.len() - 9);
                    def_type = InvSectionDefType::Children;
                    self.children_def = true;
                } else {
                    def_type = InvSectionDefType::Hosts;
                }

                let path = match self.children.first_key_for(&name) {
                    None => name.clone(),
                    Some(s) => format!("{}:{}", s, name).to_string(),
                };

                let outline = match self.outlines.get_mut(&path) {
                    None => {
                        let _outline = SectionOutline::new(path.clone());
                        self.outlines.insert(_outline.path.clone(), _outline);
                        self.outlines.get_mut(&path).unwrap()
                    }
                    Some(_outline) => _outline,
                };

                line_buf = match def_type {
                    InvSectionDefType::Children => &mut outline.children,
                    InvSectionDefType::Hosts => &mut outline.hosts,
                };
                last_path = path;
            } else {
                line_buf.push(line.to_string());
            }
        }
    }

    /// Returns a new inventory from a string.
    pub fn inv_from_string(content: String) -> Result<Inventory, UndefinedSectionError> {
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
