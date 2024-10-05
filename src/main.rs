/** CLI util to tree from inputs of lines of form `#### <name>`.
 */
use clap::{arg, value_parser, Command};
use fstree::{FSTreeMap, Node};
use std::cmp::max;
use std::fmt::Result;
use std::io;
use std::{path::PathBuf, process};

use ratatui::{
    crossterm::event::{self, KeyCode, KeyEventKind},
    prelude::*,
    style::Stylize,
    widgets::Paragraph,
    widgets::*,
    DefaultTerminal,
};

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

fn run_terminal(fs: &FSTreeMap<u64>, prefix: &str, depth: usize, human: bool) -> io::Result<()> {
    let mut terminal = ratatui::init();
    let cleared = terminal.clear()?;

    let mut state = ListState::default().with_selected(Some(0));

    loop {
        terminal.draw(|frame| {
            let mut entries: Vec<String> = vec![];

            // Recursively draw tree up to depth:
            fn add_to_entries(
                node: &Box<Node<u64>>,
                depth: usize,
                max_depth: usize,
                entries: &mut Vec<String>,
                human: bool,
            ) {
                if depth > max_depth {
                    return;
                }

                let indent = "  ".repeat(depth);
                let size = node.value_reduce(0, |a, b| a + b);
                let size = if human {
                    pretty_filesize(size)
                } else {
                    size.to_string()
                };
                entries.push(format!("{}{}: {}", indent, node.get_name(), size));

                match node.iter_children() {
                    Ok(child_iter) => child_iter.for_each(|child| {
                        add_to_entries(&child, depth + 1, max_depth, entries, human);
                    }),
                    Err(_) => {}
                }
            }

            add_to_entries(&fs.root, 0, depth, &mut entries, human);

            let list = List::new(entries)
                .block(Block::bordered().title(prefix))
                .highlight_style(Style::new().add_modifier(Modifier::REVERSED))
                .highlight_symbol(">>")
                .repeat_highlight_symbol(true);

            frame.render_stateful_widget(list, frame.area(), &mut state);
        })?;

        if let event::Event::Key(key) = event::read().unwrap() {
            if key.kind == KeyEventKind::Press && key.code == KeyCode::Char('q') {
                return Ok(());
            } else {
                match key.code {
                    KeyCode::Up => {
                        state.select_previous();
                    }
                    KeyCode::Down => {
                        state.select_next();
                    }
                    _ => {}
                }
            }
        }
    }

    ratatui::restore();
}
