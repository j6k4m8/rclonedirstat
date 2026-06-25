# RCloneDirStat

RCloneDirStat is a command-line tool for analyzing directory statistics in an [rclone](https://rclone.org/) remote. It can sum up file sizes and build a tree structure from a list of files and their sizes.

It is inspired by [WinDirStat](https://windirstat.net/) and [Disk Inventory X](https://www.derlien.com/), and of course [dust](https://github.com/bootandy/dust).

## Features

-   [x] Sum up file sizes with an optional prefix filter.
-   [x] Display file sizes in human-readable format (or raw byte count).
-   [x] Build a tree structure from a list of files and their sizes
-   [x] Navigate the tree interactively with a command-line interface

## Examples

### Simple tree

Suppose you had a Google Drive remote in `rclone` called `gdrive:`. We could,

-   list the contents of the remote at a resolution of top level directories,
-   and draw a tree with size stats,
-   and in a human-readable format

```bash
rclone ls gdrive: | cargo run -- - --human --depth 1 tree
```

```
/: 41.266 GB
  simple-reimann-proof.pdf: 1004.654 KB
  spambot_source/: 60.674 KB
  hotttt-fish-pics/: 40.205 GB
```

You might also want to save the rclone ls results to disk and then operate on them so that you don't need to pull it down fresh every time:

```bash
rclone ls gdrive: > my-rclone-ls.txt
cargo run -- my-rclone-ls.txt --human --depth 2 tree
```

### Interactive tree

You can also run in interactive mode by passing the `run` subcommand instead of the `tree` subcommand.

In this case, depth args are ignored as you can manipulate the depth interactively.

| Command                   | Shortcut         |
| ------------------------- | ---------------- |
| Move upward in the tree   | <kbd>↑</kbd>     |
| Move downward in the tree | <kbd>↓</kbd>     |
| Collapse a directory      | <kbd>←</kbd>     |
| Expand a directory        | <kbd>→</kbd>     |
| Select a directory        | <kbd>Enter</kbd> |
| Quit interactive mode     | <kbd>q</kbd>     |

## Development

This crate depends on `j6k4m8/fstree` from GitHub. Cargo fetches it automatically during builds and tests.
