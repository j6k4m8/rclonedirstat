/** CLI util to tree from inputs of lines of form `#### <name>`.
 */
use clap::{arg, value_parser, Command};
use fstree::{FSTreeMap, Node};
use std::cmp::max;
use std::io;
use std::{path::PathBuf, process};

use ratatui::{
    crossterm::event::{self, KeyCode, KeyEventKind},
    // prelude::*,
    // widgets::*,
};
use tui_tree_widget::{Tree, TreeItem, TreeState};

/// Parse the input from the user, either from a file or from stdin.
///
/// The input should be a list of lines, where each line is a file size (in
/// bytes) and a file path, separated by a space. The file path should be the
/// full path of the file, starting from the root of the directory tree. For
/// example, the following input:
///
/// ```
///  100 /home/user/file.txt
/// 1200 /home/user/dir/file2.txt
/// ```
///
/// Note that the sizes are right-aligned, and the file paths are left-aligned.
fn parse_input(stream: &mut dyn std::io::BufRead) -> Vec<(u64, String)> {
    // Read the input from the user:
    let mut listing = Vec::new();
    let mut line = String::new();
    // while std::io::stdin().read_line(&mut line).unwrap() > 0 {
    while stream.read_line(&mut line).unwrap() > 0 {
        // Parse the line:
        let parts: Vec<&str> = line.trim().split(" ").collect();
        // Combine the parts after the size:
        let path = "/".to_owned() + &parts[1..].join(" ");
        let size = parts[0].parse::<i64>().unwrap();
        let size = max(size as i64, 0) as u64;
        listing.push((size, path));
        line.clear();
    }
    listing
}

fn cli() -> Command {
    Command::new("rclonedirstat")
        .version("0.1.0")
        .about("Prints the sizes of a directory tree.")
        .subcommand(
            Command::new("sum")
                .about("Prints the sum of the sizes of the files in the directory tree."),
        )
        .subcommand(Command::new("run").about("Prints the directory tree."))
        .subcommand(Command::new("tree").about("Prints the directory tree."))
        .args([
            arg!([file] "The file to process").default_value("-"),
            arg!([prefix] "The prefix to search for").default_value(" "),
            arg!(--depth <DEPTH> "The depth of three to unfold")
                .default_value("0")
                .value_parser(value_parser!(usize)),
            arg!(--human "Prints the sizes in human-readable format"),
        ])
}

fn pretty_filesize(size_bytes: u64) -> String {
    let units = ["B", "KB", "MB", "GB", "TB", "PB", "EB", "ZB", "YB"];
    let size = size_bytes as f64;
    let i = max(0, (size.ln() / 1024_f64.ln()).floor() as i32);
    let size = size / 1024_f64.powi(i);
    format!("{:.3} {}", size, units[i as usize])
}

fn print_tree(node: &Box<Node<u64>>, depth: usize, max_depth: usize) {
    if depth > max_depth {
        return;
    }

    let indent = "  ".repeat(depth);
    println!(
        "{}{}: {}",
        indent,
        node.get_name(),
        pretty_filesize(node.value_reduce(0, |a, b| a + b))
    );

    match node.iter_children() {
        Ok(child_iter) => child_iter.for_each(|child| {
            print_tree(&child, depth + 1, max_depth);
        }),
        Err(_) => {}
    }
}

fn main() {
    // Get the command line arguments:
    let matches = cli().get_matches();
    let listing;

    let name = matches.get_one::<String>("file");

    if name.is_some() && name.unwrap() != "-" {
        let path = PathBuf::from(name.unwrap());
        let file = std::fs::File::open(&path).unwrap_or_else(|_| {
            eprintln!("Could not open file: {}", path.display());
            process::exit(1);
        });
        let mut reader = std::io::BufReader::new(file);
        listing = parse_input(&mut reader);
    } else {
        let stdin = std::io::stdin();
        let mut reader = stdin.lock();
        listing = parse_input(&mut reader);
    }

    let prefix = matches.get_one::<String>("prefix").unwrap();
    let human = *matches.get_one::<bool>("human").unwrap();
    let depth = *matches.get_one::<usize>("depth").unwrap();

    match matches.subcommand() {
        Some(("sum", _)) => {
            let total_size: u64 = listing
                .iter()
                .filter(|(_, path)| path.starts_with(prefix))
                .map(|(size, _)| size)
                .sum();
            if human {
                println!("{}", pretty_filesize(total_size as u64));
            } else {
                println!("{}", total_size);
            }
        }
        Some(("tree", _)) => {
            let mut fs: FSTreeMap<u64> = FSTreeMap::new();

            listing
                .iter()
                .filter(|(_, path)| path.starts_with(prefix))
                .filter(|(size, _)| *size > 0)
                .for_each(|(size, path)| {
                    // let path_splits: Vec<String> = path.split("/").map(|s| s.to_string()).collect();
                    // builder.push(path_splits, *size as u64);
                    fs.insert_with_parents(path, *size);
                });

            for node in fs.iter_children(None).unwrap() {
                print_tree(node, 0, depth);
            }
        }
        Some(("run", _)) => {
            let mut fs: FSTreeMap<u64> = FSTreeMap::new();

            listing
                .iter()
                .filter(|(_, path)| path.starts_with(prefix))
                .filter(|(size, _)| *size > 0)
                .for_each(|(size, path)| {
                    // let path_splits: Vec<String> = path.split("/").map(|s| s.to_string()).collect();
                    // builder.push(path_splits, *size as u64);
                    fs.insert_with_parents(path, *size);
                });

            _ = run_terminal(&fs, prefix, depth, human);
        }
        _ => {
            eprintln!("No subcommand provided");
            process::exit(1);
        }
    }
}

fn run_terminal(fs: &FSTreeMap<u64>, _prefix: &str, _depth: usize, human: bool) -> io::Result<()> {
    let mut terminal = ratatui::init();
    // let cleared = terminal.clear()?;

    let mut entries: Vec<TreeItem<String>> = vec![];

    // Instead of pushing a flat list to entries, we need to push a tree structure:
    let _root = TreeItem::new("root".to_string(), "root", vec![]).unwrap();
    // entries.push(root);

    fn node_to_treeitem(node: &Box<Node<u64>>, human: bool) -> TreeItem<String> {
        let size = node.value_reduce(0, |a, b| a + b);
        let size = if human {
            pretty_filesize(size)
        } else {
            size.to_string()
        };
        let mut children = vec![];

        match node.iter_children() {
            Ok(child_iter) => child_iter.for_each(|child| {
                let child_name = child.get_name().to_string();
                if !children
                    .iter()
                    .any(|c: &TreeItem<'_, String>| child_name.eq(c.identifier()))
                {
                    children.push(node_to_treeitem(&child, human));
                } else {
                    // panic!("Already saw {}", child_name)
                }
            }),
            Err(_) => {}
        }

        let new_tree_item_result = TreeItem::new(
            node.get_name().to_string(),
            format!("{} ({})", node.get_name().to_string(), size),
            children,
        );
        match new_tree_item_result {
            Ok(res) => return res,
            Err(e) => {
                println!("{:?}", e);
                panic!("Failed on directory / node {:?}:\n \n\n", node.get_name(),)
            }
        }
    }

    fs.root.iter_children().unwrap().for_each(|child| {
        entries.push(node_to_treeitem(&child, human));
    });

    let mut tree_state: TreeState<_> = TreeState::<String>::default();

    loop {
        terminal.draw(|frame| {
            let list = Tree::new(&entries).unwrap().highlight_style(
                ratatui::style::Style::default().add_modifier(ratatui::style::Modifier::BOLD),
            );
            frame.render_stateful_widget(list, frame.area(), &mut tree_state);
        })?;

        if let event::Event::Key(key) = event::read().unwrap() {
            if key.kind == KeyEventKind::Press && key.code == KeyCode::Char('q') {
                break;
            } else {
                match key.code {
                    KeyCode::Up => {
                        tree_state.key_up();
                    }
                    KeyCode::Down => {
                        tree_state.key_down();
                    }
                    KeyCode::Left => {
                        tree_state.key_left();
                    }
                    KeyCode::Right => {
                        tree_state.key_right();
                    }
                    KeyCode::Enter => {
                        tree_state.toggle_selected();
                    }
                    _ => {}
                }
            }
        }
    }

    ratatui::restore();
    return Ok(());
}
