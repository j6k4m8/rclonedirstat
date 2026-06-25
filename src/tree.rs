use fstree::Node;
use std::cmp::max;

pub struct SizedNode {
    pub name: String,
    pub size: u64,
    pub children: Vec<SizedNode>,
}

pub fn pretty_filesize(size_bytes: u64) -> String {
    let units = ["B", "KB", "MB", "GB", "TB", "PB", "EB", "ZB", "YB"];
    let size = size_bytes as f64;
    let i = max(0, (size.ln() / 1024_f64.ln()).floor() as i32);
    let size = size / 1024_f64.powi(i);
    format!("{:.3} {}", size, units[i as usize])
}

pub fn sized_node_from_fs(node: &Node<u64>) -> SizedNode {
    match node {
        Node::File { name, size } => SizedNode {
            name: name.clone(),
            size: *size,
            children: vec![],
        },
        Node::Directory { name, children } => {
            let children: Vec<SizedNode> = children
                .iter()
                .map(|child| sized_node_from_fs(child.as_ref()))
                .collect();
            let size = children.iter().map(|child| child.size).sum();
            SizedNode {
                name: name.clone(),
                size,
                children,
            }
        }
    }
}

pub fn print_tree(node: &SizedNode, depth: usize, max_depth: usize) {
    if depth > max_depth {
        return;
    }

    let indent = "  ".repeat(depth);
    println!("{}{}: {}", indent, node.name, pretty_filesize(node.size));

    node.children
        .iter()
        .for_each(|child| print_tree(child, depth + 1, max_depth));
}
