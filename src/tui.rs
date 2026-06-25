use crate::tree::{pretty_filesize, sized_node_from_fs, SizedNode};
use fstree::FSTreeMap;
use ratatui::crossterm::event::{self, KeyCode, KeyEventKind};
use std::io;
use tui_tree_widget::{Tree, TreeItem, TreeState};

struct TerminalRestore;

impl Drop for TerminalRestore {
    fn drop(&mut self) {
        ratatui::restore();
    }
}

pub fn run_terminal(fs: &FSTreeMap<u64>, human: bool) -> io::Result<()> {
    let mut terminal = ratatui::init();
    let _restore_terminal = TerminalRestore;

    let mut entries: Vec<TreeItem<'static, String>> = vec![];

    for child in fs.root.iter_children().unwrap() {
        let child = sized_node_from_fs(child.as_ref());
        entries.push(node_to_treeitem(&child, human)?);
    }

    let mut tree_state: TreeState<_> = TreeState::<String>::default();

    loop {
        let list = Tree::new(&entries)
            .map_err(tree_widget_error)?
            .highlight_style(
                ratatui::style::Style::default().add_modifier(ratatui::style::Modifier::BOLD),
            );

        terminal.draw(|frame| {
            frame.render_stateful_widget(list, frame.area(), &mut tree_state);
        })?;

        if let event::Event::Key(key) = event::read()? {
            if key.kind == KeyEventKind::Press && key.code == KeyCode::Char('q') {
                break;
            }

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

    Ok(())
}

fn node_to_treeitem(node: &SizedNode, human: bool) -> io::Result<TreeItem<'static, String>> {
    let size = if human {
        pretty_filesize(node.size)
    } else {
        node.size.to_string()
    };
    let mut children = vec![];

    for child in &node.children {
        if !children
            .iter()
            .any(|c: &TreeItem<'static, String>| child.display_name().eq(c.identifier()))
        {
            children.push(node_to_treeitem(child, human)?);
        }
    }

    TreeItem::new(
        node.display_name().to_string(),
        format!("{} ({})", node.display_name(), size),
        children,
    )
    .map_err(tree_widget_error)
}

fn tree_widget_error<E: std::fmt::Debug>(err: E) -> io::Error {
    io::Error::new(io::ErrorKind::InvalidData, format!("{err:?}"))
}
