use crate::error::UndefinedSectionError;

use std::collections::{HashMap, HashSet};

pub trait SectionContainer<'a> {
    /// Returns all the sections that exist directly within this one.
    fn children(&'a self) -> Vec<&'a Section>;

    /// Returns all the sections that are descended from this one.
    fn descendants(&'a self) -> Vec<&'a Section> {
        return self.children().iter().map(|sec| sec.descendants())
        .flatten().collect()
    }

    /// Returns all hosts defined directly within this section. 
    fn child_hosts(&'a self) -> Vec<&'a str>;

    /// Returns all hosts descended from this one.
    fn descended_hosts(&'a self) -> Vec<&'a str> {
        return self.descendants().iter().map(|sec| sec.child_hosts())
            .flatten().collect()
    }
}

#[derive(Debug, Hash, PartialEq, Eq)]
struct SectionOutline {
    pub name: String,
    pub path: String,
    pub children: Vec<String>,
    pub hosts: Vec<String>
}
impl SectionOutline {
    pub fn new(path: String) -> SectionOutline {
        let name: String;
        if path.contains(':') {
            name = path.rsplit_once(':').unwrap().1.to_string();
        } else {
            name = path.clone();
        }
        println!("OUTLINE-- name: {}; path: {};", &name, &path);
        SectionOutline { name, path, children:  Vec::new(), hosts: Vec::new() } 
    }
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
    children: HashMap<String, Section>
}

impl Section {
    pub fn new(path: String) -> Section {
        let name: String;
        if path.contains(':') {
            name = path.rsplit_once(':').unwrap().1.to_string();
        } else {
            name = path.clone();
        }
        return Section { name, path, hosts: Vec::new(), children: HashMap::new() }
    }
}

impl<'a> SectionContainer<'a> for Section {
    fn children(&'a self) -> Vec<&'a Section> {
        return self.children.values().collect()
    }

    fn child_hosts(&'a self) -> Vec<&'a str> {
        return self.hosts.iter().map(|s| &**s).collect()
    }
} 

#[derive(Debug)]
pub struct Inventory {
    /// Hostnames defined in the inventory outside of a section.
    hosts: Vec<String>,
    /// Top-level sections in the inventory.
    sections: HashMap<String, Section>
}

impl<'a> SectionContainer<'a> for Inventory {
    fn children(&'a self) -> Vec<&'a Section> {
        return self.sections.values().collect()
    }

    fn child_hosts(&'a self) -> Vec<&'a str> {
        return self.hosts.iter().map(|s| &**s).collect()
    }
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
    children_def: bool
}

impl InventoryParser {
    /// Recursively resolves this outline and its children to sections.
    fn resolve_outline(&self, outline: &SectionOutline) -> Result<(Section, HashSet<String>), UndefinedSectionError> {
        let mut section = Section::new(outline.path.to_string());
        let mut child_names = HashSet::new();
        for child_name in &outline.children {
            child_names.insert(child_name.clone());
            match self.outlines.get(child_name) {
                None => {return Err(UndefinedSectionError {name: child_name.to_string()})},
                Some(child_outline) => {
                    let (child, _child_names) = self.resolve_outline(child_outline)?;
                    child_names.extend(_child_names);
                    section.children.insert(child_name.to_string(), child);
                }
            }
        }

        section.hosts = outline.hosts.clone();
        return Ok((section, child_names))
    }

    /// Creates an inventory from stored data.
    fn to_inv(&self) -> Result<Inventory, UndefinedSectionError> {
        let mut sections = HashMap::new();
        for outline in self.outlines.values() {
            if !outline.path.contains(':') {
                let (section, _child_names) = self.resolve_outline(outline)?;
                sections.insert(section.name.clone(), section);
            }
        }

        return Ok(Inventory { hosts: self.hosts.clone(), sections })
    }

    fn parse(&mut self) -> Result<Inventory, UndefinedSectionError> {
        let mut line_buf: &mut Vec<String> = &mut self.hosts;
        let mut last_path = "".to_string();
        for line in self.content.lines() {
            if line.starts_with("#") | line.trim().is_empty() {
                continue;
            } else if line.starts_with("[") {
                let mut name = line.trim()[1..line.len() - 1].to_string();
                if self.children_def == true { // leaving children def
                    self.children.insert(last_path, line_buf.clone());
                    self.children_def = false;
                }

                let def_type: &'static str;
                if name.ends_with(":children") {
                    name.truncate(name.len() - 9);
                    def_type = "child";
                    self.children_def = true;
                } else {
                    def_type = "host";
                }

                let path = match self.children.first_key_for(&name) {
                        None => name.clone(),
                        Some(s) => format!("{}:{}", s, name).to_string()
                };
                println!("name: {}; path: {}", &name, &path);
                let outline = SectionOutline::new(path.clone());
                self.outlines.insert(name.clone(), outline);

                line_buf = match def_type {
                    "child" => &mut self.outlines.get_mut(&name).unwrap().children,
                    "host" => &mut self.outlines.get_mut(&name).unwrap().hosts,
                    _ => panic!("Unknown definition type: {}", def_type)
                };
                last_path = path;
            } else {
                line_buf.push(line.to_string());
            }
        }
        return self.to_inv()
    }

    /// Returns a new inventory from a string.
    pub fn inv_from_string(content: String) -> Result<Inventory, UndefinedSectionError> {
        return InventoryParser { content, 
            outlines: HashMap::new(), 
            hosts: Vec::new(), 
            children: HashMap::new(), 
            children_def: false 
        }.parse()
    }
}



trait Finder<T> {
    fn first_key_for(&self, val: &T) -> Option<&T>;
}

impl<T: Eq> Finder<T> for HashMap<T, Vec<T>> {
    fn first_key_for(&self, val: &T) -> Option<&T> {
        for (key, values) in self {
            if values.contains(val) {
                return Some(key)
            }
        }
        return None
    }
}