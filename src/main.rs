/** CLI util to tree from inputs of lines of form `#### <name>`.
 */
mod tree;
mod tui;

use clap::{arg, value_parser, Command};
use std::cmp::max;
use std::collections::BTreeMap;
use std::{path::PathBuf, process};

use tree::{build_tree, pretty_filesize, print_tree, sized_node_from_fs};

fn parse_listing_line(line: &str) -> Option<(i64, String)> {
    let trimmed = line.trim_end();
    let trimmed = trimmed.trim_start();
    if trimmed.is_empty() {
        return None;
    }

    let Some(size_end) = trimmed.find(char::is_whitespace) else {
        eprintln!("Skipping malformed line without path: {trimmed}");
        return None;
    };

    let size_text = &trimmed[..size_end];
    let path_text = trimmed[size_end..].trim_start();
    if path_text.is_empty() {
        eprintln!("Skipping malformed line without path: {trimmed}");
        return None;
    }

    let Ok(size) = size_text.parse::<i64>() else {
        eprintln!("Skipping malformed line with invalid size: {trimmed}");
        return None;
    };

    let path = if path_text.starts_with('/') {
        path_text.to_string()
    } else {
        "/".to_owned() + path_text
    };
    Some((size, path))
}

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
    let mut listing = Vec::new();
    let mut line = String::new();
    while stream.read_line(&mut line).unwrap() > 0 {
        if let Some((size, path)) = parse_listing_line(&line) {
            listing.push((max(size, 0) as u64, path));
        }
        line.clear();
    }
    listing
}

fn relative_path<'a>(path: &'a str, prefix: &str) -> &'a str {
    if prefix == "/" {
        path.trim_start_matches('/')
    } else {
        path.strip_prefix(prefix)
            .unwrap_or(path)
            .trim_start_matches('/')
    }
}

fn group_for_depth(path: &str, prefix: &str, depth: usize) -> String {
    let relative_path = relative_path(path, prefix);
    let group = relative_path
        .split('/')
        .filter(|part| !part.is_empty())
        .take(depth)
        .collect::<Vec<_>>()
        .join("/");

    if group.is_empty() {
        prefix.to_string()
    } else {
        group
    }
}

fn sum_input(stream: &mut dyn std::io::BufRead, prefix: &str) -> u64 {
    let mut total_size = 0;
    let mut line = String::new();

    while stream.read_line(&mut line).unwrap() > 0 {
        if let Some((size, path)) = parse_listing_line(&line) {
            if path.starts_with(prefix) {
                total_size += max(size, 0) as u64;
            }
        }
        line.clear();
    }

    total_size
}

fn sum_groups_input(
    stream: &mut dyn std::io::BufRead,
    prefix: &str,
    depth: usize,
) -> Vec<(String, u64)> {
    let mut sums = BTreeMap::new();
    let mut line = String::new();

    while stream.read_line(&mut line).unwrap() > 0 {
        if let Some((size, path)) = parse_listing_line(&line) {
            if path.starts_with(prefix) {
                let group = group_for_depth(&path, prefix, depth);
                *sums.entry(group).or_insert(0) += max(size, 0) as u64;
            }
        }
        line.clear();
    }

    sums.into_iter().collect()
}

fn count_input(stream: &mut dyn std::io::BufRead, prefix: &str) -> usize {
    let mut count = 0;
    let mut line = String::new();

    while stream.read_line(&mut line).unwrap() > 0 {
        if let Some((size, path)) = parse_listing_line(&line) {
            if size >= 0 && path.starts_with(prefix) {
                count += 1;
            }
        }
        line.clear();
    }

    count
}

fn count_groups_input(
    stream: &mut dyn std::io::BufRead,
    prefix: &str,
    depth: usize,
) -> Vec<(String, usize)> {
    let mut counts = BTreeMap::new();
    let mut line = String::new();

    while stream.read_line(&mut line).unwrap() > 0 {
        if let Some((size, path)) = parse_listing_line(&line) {
            if size >= 0 && path.starts_with(prefix) {
                let group = group_for_depth(&path, prefix, depth);
                *counts.entry(group).or_insert(0) += 1;
            }
        }
        line.clear();
    }

    counts.into_iter().collect()
}

fn with_input_reader<T>(
    name: Option<&String>,
    f: impl FnOnce(&mut dyn std::io::BufRead) -> T,
) -> T {
    if name.is_some() && name.unwrap() != "-" {
        let path = PathBuf::from(name.unwrap());
        let file = std::fs::File::open(&path).unwrap_or_else(|_| {
            eprintln!("Could not open file: {}", path.display());
            process::exit(1);
        });
        let mut reader = std::io::BufReader::new(file);
        f(&mut reader)
    } else {
        let stdin = std::io::stdin();
        let mut reader = stdin.lock();
        f(&mut reader)
    }
}

fn cli() -> Command {
    Command::new("rclonedirstat")
        .version("0.1.0")
        .about("Prints the sizes of a directory tree.")
        .subcommand(
            Command::new("sum")
                .about("Prints the sum of the sizes of the files in the directory tree."),
        )
        .subcommand(
            Command::new("count")
                .about("Prints the count of files with real sizes in the directory tree."),
        )
        .subcommand(Command::new("run").about("Prints the directory tree."))
        .subcommand(Command::new("tree").about("Prints the directory tree."))
        .args([
            arg!([file] "The file to process").default_value("-"),
            arg!([prefix] "The prefix to search for").default_value("/"),
            arg!(--depth <DEPTH> "The depth of three to unfold")
                .default_value("0")
                .value_parser(value_parser!(usize)),
            arg!(--human "Prints the sizes in human-readable format"),
        ])
}

fn main() {
    // Get the command line arguments:
    let matches = cli().get_matches();

    let name = matches.get_one::<String>("file");
    let prefix = matches.get_one::<String>("prefix").unwrap();
    let human = *matches.get_one::<bool>("human").unwrap();
    let depth = *matches.get_one::<usize>("depth").unwrap();

    match matches.subcommand() {
        Some(("sum", _)) => {
            if depth == 0 {
                let total_size = with_input_reader(name, |reader| sum_input(reader, prefix));
                if human {
                    println!("{}", pretty_filesize(total_size));
                } else {
                    println!("{}", total_size);
                }
            } else {
                let sums =
                    with_input_reader(name, |reader| sum_groups_input(reader, prefix, depth));
                sums.iter().for_each(|(group, size)| {
                    if human {
                        println!("{}: {}", group, pretty_filesize(*size));
                    } else {
                        println!("{}: {}", group, size);
                    }
                });
            }
        }
        Some(("count", _)) => {
            if human {
                eprintln!("Warning: --human has no effect for count.");
            }

            if depth == 0 {
                let count = with_input_reader(name, |reader| count_input(reader, prefix));
                println!("{}", count);
            } else {
                let counts =
                    with_input_reader(name, |reader| count_groups_input(reader, prefix, depth));
                counts.iter().for_each(|(group, count)| {
                    println!("{}: {}", group, count);
                });
            }
        }
        Some(("tree", _)) => {
            let listing = with_input_reader(name, parse_input);
            let fs = build_tree(&listing, prefix);

            for node in fs
                .iter_children(None)
                .unwrap()
                .map(|node| sized_node_from_fs(node.as_ref()))
            {
                print_tree(&node, 0, depth);
            }
        }
        Some(("run", _)) => {
            let listing = with_input_reader(name, parse_input);
            let fs = build_tree(&listing, prefix);

            if let Err(err) = tui::run_terminal(&fs, human) {
                eprintln!("Interactive terminal failed: {err}");
                process::exit(1);
            }
        }
        _ => {
            eprintln!("No subcommand provided");
            process::exit(1);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Cursor;

    #[test]
    fn parse_input_handles_repeated_spaces_and_blank_lines() {
        let input = b"
            100    DIR_1/A.txt
            -7 DIR_1/B with spaces.txt
            3 /already/rooted.txt

            nope DIR_1/C.txt
            5
        ";
        let mut cursor = Cursor::new(input);

        assert_eq!(
            parse_input(&mut cursor),
            vec![
                (100, "/DIR_1/A.txt".to_string()),
                (0, "/DIR_1/B with spaces.txt".to_string()),
                (3, "/already/rooted.txt".to_string())
            ]
        );
    }

    #[test]
    fn sum_input_streams_matching_prefix() {
        let input = b"
            100 DIR_1/A.txt
            7 DIR_2/B.txt
            -3 DIR_1/C.txt
        ";
        let mut cursor = Cursor::new(input);

        assert_eq!(sum_input(&mut cursor, "/DIR_1"), 100);
    }

    #[test]
    fn count_input_excludes_negative_sizes() {
        let input = b"
            100 DIR_1/A.txt
            0 DIR_1/empty.txt
            -1 DIR_1/google-doc
            7 DIR_2/B.txt
        ";
        let mut cursor = Cursor::new(input);

        assert_eq!(count_input(&mut cursor, "/DIR_1"), 2);
    }

    #[test]
    fn sum_groups_by_positive_depth() {
        let input = b"
            2 root.txt
            3 DIR_1/A.txt
            5 DIR_1/X/B.txt
        ";
        let mut cursor = Cursor::new(input);

        assert_eq!(
            sum_groups_input(&mut cursor, "/", 2),
            vec![
                ("DIR_1/A.txt".to_string(), 3),
                ("DIR_1/X".to_string(), 5),
                ("root.txt".to_string(), 2),
            ]
        );
    }

    #[test]
    fn count_groups_by_positive_depth() {
        let input = b"
            2 root.txt
            3 DIR_1/A.txt
            5 DIR_1/X/B.txt
            -1 DIR_1/X/google-doc
        ";
        let mut cursor = Cursor::new(input);

        assert_eq!(
            count_groups_input(&mut cursor, "/", 2),
            vec![
                ("DIR_1/A.txt".to_string(), 1),
                ("DIR_1/X".to_string(), 1),
                ("root.txt".to_string(), 1),
            ]
        );
    }

    #[test]
    fn cli_defaults_prefix_to_root() {
        let matches = cli()
            .try_get_matches_from(["rclonedirstat", "basic.txt", "tree"])
            .unwrap();

        assert_eq!(matches.get_one::<String>("prefix").unwrap(), "/");
        assert!(matches.subcommand_matches("tree").is_some());
    }
}
