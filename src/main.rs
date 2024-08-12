use std::cmp::max;
/** CLI util to tree from inputs of lines of form `#### <name>`.
 */

// use std::env;
use std::{path::PathBuf, process};
use clap::{arg, Command};
use trie_rs::iter::SearchIter;
use trie_rs::map::Trie;
use trie_rs::map::TrieBuilder;
use trie_rs::try_collect::TryCollect;


/**
 * Parse the input from the user, either from a file or from stdin.
 *
 * The input should be a list of lines, where each line is a file size (in
 * bytes) and a file path, separated by a space. The file path should be the
 * full path of the file, starting from the root of the directory tree. For
 * example, the following input:
 *
 *  100 /home/user/file.txt
 * 1200 /home/user/dir/file2.txt
 *
 * Note that the sizes are right-aligned, and the file paths are left-aligned.
 */
fn parse_input(stream: &mut dyn std::io::BufRead) -> Vec<(i64, String)> {
    // Read the input from the user:
    let mut listing = Vec::new();
    let mut line = String::new();
    // while std::io::stdin().read_line(&mut line).unwrap() > 0 {
    while stream.read_line(&mut line).unwrap() > 0 {
        // Parse the line:
        let parts: Vec<&str> = line.trim().split(" ").collect();
        // Combine the parts after the size:
        let path = parts[1..].join(" ");
        let size: i64 = parts[0].parse().unwrap();
        listing.push((size, path));
        line.clear();
    }
    listing
}


fn cli() -> Command {
    Command::new("rclonedirstat")
        .version("0.1.0")
        .about("Prints the sizes of a directory tree.")
        .arg(arg!([file] "The file to process").default_value("-")
            // .value_parser(clap::value_parser!(PathBuf))
        )
        .subcommand(Command::new("sum")
            .about("Prints the sum of the sizes of the files in the directory tree."))
        .subcommand(Command::new("tree")
            .about("Prints the directory tree."))
}

fn pretty_filesize(size_bytes: u64) -> String {
    let units = ["B", "KB", "MB", "GB", "TB", "PB", "EB", "ZB", "YB"];
    let size = size_bytes as f64;
    let i = max(0, (size.ln() / 1024_f64.ln()).floor() as i32);
    let size = size / 1024_f64.powi(i);
    format!("{:.2} {}", size, units[i as usize])
}

fn main() {
    // Get the command line arguments:
    let matches = cli().get_matches();
    let listing;

    let name = matches.get_one::<String>("file");

    if name.is_some() && name.unwrap() != "-" {
        let path = PathBuf::from(name.unwrap());
        let file = std::fs::File::open
            (&path).unwrap_or_else(|_| {
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

    let  prefix = "DIR_1";

    match matches.subcommand() {
        Some(("sum", _)) => {
            let total_size: i64 = listing.iter().map(|(size, _)| size).sum();
            println!("Total size: {}", total_size);
        }
        Some(("tree", _)) => {
            // let trie = create_tree_from_listing(listing);

            let mut builder = TrieBuilder::new();

            listing.iter().for_each(|(size, path)| {
                let path_splits: Vec<String> = path.split("/").map(|s| s.to_string()).collect();
                builder.push(path_splits, *size as u64);
            });

            let trie = builder.build();

            // let mut search = trie.inc_search();
            let qry: Vec<String> = prefix.split("/").map(|s| s.to_string()).collect::<Vec<String>>();
            let results_iter: SearchIter<String, u64, Vec<String>, _> = trie.predictive_search(qry);
            let mut size_sum: u64 = 0;
            let mut fcount = 0;
            results_iter.for_each(|(path, size)| {
                size_sum += size;
                fcount += 1;
            });
            println!("Total size: {}", pretty_filesize(size_sum));
            println!("Total files: {}", fcount);

        }
        _ => {
            eprintln!("No subcommand provided");
            process::exit(1);
        }
    }
}
