/** CLI util to tree from inputs of lines of form `#### <name>`.
 */
mod tree;
mod tui;

use clap::{arg, value_parser, Command};
use std::cmp::max;
use std::{path::PathBuf, process};

use tree::{build_tree, pretty_filesize, print_tree, sized_node_from_fs};

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
        let trimmed = line.trim_end();
        let trimmed = trimmed.trim_start();
        if trimmed.is_empty() {
            line.clear();
            continue;
        }

        let Some(size_end) = trimmed.find(char::is_whitespace) else {
            eprintln!("Skipping malformed line without path: {trimmed}");
            line.clear();
            continue;
        };

        let size_text = &trimmed[..size_end];
        let path_text = trimmed[size_end..].trim_start();
        if path_text.is_empty() {
            eprintln!("Skipping malformed line without path: {trimmed}");
            line.clear();
            continue;
        }

        let Ok(size) = size_text.parse::<i64>() else {
            eprintln!("Skipping malformed line with invalid size: {trimmed}");
            line.clear();
            continue;
        };

        let path = if path_text.starts_with('/') {
            path_text.to_string()
        } else {
            "/".to_owned() + path_text
        };
        let size = max(size, 0) as u64;
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

    let listing = if name.is_some() && name.unwrap() != "-" {
        let path = PathBuf::from(name.unwrap());
        let file = std::fs::File::open(&path).unwrap_or_else(|_| {
            eprintln!("Could not open file: {}", path.display());
            process::exit(1);
        });
        let mut reader = std::io::BufReader::new(file);
        parse_input(&mut reader)
    } else {
        let stdin = std::io::stdin();
        let mut reader = stdin.lock();
        parse_input(&mut reader)
    };

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
                println!("{}", pretty_filesize(total_size));
            } else {
                println!("{}", total_size);
            }
        }
        Some(("tree", _)) => {
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
    fn cli_defaults_prefix_to_root() {
        let matches = cli()
            .try_get_matches_from(["rclonedirstat", "basic.txt", "tree"])
            .unwrap();

        assert_eq!(matches.get_one::<String>("prefix").unwrap(), "/");
        assert!(matches.subcommand_matches("tree").is_some());
    }
}
