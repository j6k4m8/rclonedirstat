# RCloneDirStat

RCloneDirStat is a command-line tool for analyzing directory statistics in an [rclone](https://rclone.org/) remote. It can sum up file sizes and build a tree structure from a list of files and their sizes.

It is inspired by [WinDirStat](https://windirstat.net/) and [Disk Inventory X](https://www.derlien.com/).

## Features

-   [x] Sum up file sizes with an optional prefix filter.
-   [x] Display file sizes in human-readable format (or raw byte count).
-   [x] Build a tree structure from a list of files and their sizes

## Roadmap

-   [ ] Navigate the tree interactively with a command-line interface

## Examples

Suppose you had a Google Drive remote in `rclone` called `gdrive:`. We could,

-   list the contents of the remote at a resolution of top level directories,
-   and draw a tree with size stats,
-   and in a human-readable format

```bash
rclone ls gdrive: | cargo run - '/' --human --depth 1 tree
```

```
: 41.266 GB
  simple-reimann-proof.pdf: 1004.654 KB
  spambot_source/: 60.674 KB
  hotttt-fish-pics/: 40.205 GB
```
