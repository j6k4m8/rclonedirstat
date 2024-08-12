use std::{cmp::Eq, hash::Hash, io::BufRead};


trait TreeNode {
    fn name(&self) -> &str;
    fn size(&mut self) -> u64;
    fn is_directory(&self) -> bool;
    fn try_as_directory(&mut self) -> Option<&mut Directory> {
        None
    }
}

impl std::fmt::Debug for dyn TreeNode {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "{}", self.name())
    }
}

#[derive(Debug)]
struct Directory {
    name: String,
    children: Vec<Box<dyn TreeNode>>,
    size_is_cached: bool,
    cached_size: u64,
}

impl Directory {
    fn new(name: String) -> Self {
        Directory {
            name,
            children: Vec::new(),
            size_is_cached: true,
            cached_size: 0,
        }
    }

    fn add_child(&mut self, child: Box<dyn TreeNode>) {
        self.children.push(child);
        self.size_is_cached = false;
    }

    fn try_as_directory(&self) -> Option<&Directory> {
        Some(self)
    }

    fn _compute_size(&mut self) {
        self.cached_size = 0;
        for child in &mut self.children {
            self.cached_size += child.size();
        }
        self.size_is_cached = true;
    }
}

impl TreeNode for Directory {
    fn name(&self) -> &str {
        &self.name
    }

    fn size(&mut self) -> u64 {
        if self.size_is_cached {
            return self.cached_size;
        }
        self._compute_size();
        self.cached_size
    }

    fn is_directory(&self) -> bool {
        true
    }

    fn try_as_directory(&mut self) -> Option<&mut Directory> {
        Some(self)
    }
}

struct File {
    name: String,
    size: u64,
}

impl File {
    fn new(name: String, size: u64) -> Self {
        File { name, size }
    }
}

impl TreeNode for File {
    fn name(&self) -> &str {
        &self.name
    }

    fn size(&mut self) -> u64 {
        self.size
    }

    fn is_directory(&self) -> bool {
        false
    }

    fn try_as_directory(&mut self) -> Option<&mut Directory> {
        None
    }
}

#[derive(Debug)]
pub struct FSTrieMap<V> {
    root: Directory,
    hashmap: std::collections::HashMap<String, V>,
}


impl<V> FSTrieMap<V>
where       V: std::fmt::Debug
{
    pub(crate) fn new() -> Self {
        FSTrieMap {
            root: Directory::new("".to_string()),
            hashmap: std::collections::HashMap::new(),
        }
    }

    /**
     * Insert into both the trie and the hashmap.
     *
     * The key in the hashmap is the FULL path. The path from the root to the
     * node in the trie is, for path "a/b/c.txt", a node called "a" with child
     * "a/b" with a child called "a/b/c.txt".
     */
    pub(crate) fn insert(&mut self, key: &str, value: V) {
        // First insert into the trie:
        self.hashmap.insert(key.to_owned(), value);
        let mut current = &mut self.root;
        for part in key.split("/") {
            if part.is_empty() {
                continue;
            }
            let mut found = false;
            for child in &mut current.children {
                if child.name() == part {
                    current = child.try_as_directory().unwrap();
                    found = true;
                    break;
                }
            }
            if !found {
                let new_dir = Box::new(Directory::new(part.to_string()));
                current.add_child(new_dir);
                current = current.children.last_mut().unwrap().try_as_directory().unwrap();
            }
        }
    }
}