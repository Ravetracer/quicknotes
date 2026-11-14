//! The notes library: folders (nested to any depth) holding notes and links
//! to files, open or not, plus the trash. This module is the logic of the
//! folder tree; the sidebar is in [`crate::notes_tree`].

use crate::store::Folder;
use std::collections::{HashMap, HashSet};

/// A note or file as listed in the tree.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Item {
    pub id: String,
    pub title: String,
    pub folder: Option<String>,
    pub trashed: bool,
    /// Open in a tab.
    pub open: bool,
    /// A file (not a note stored in QuickNotes).
    pub file: bool,
    /// A file that no longer exists.
    pub missing: bool,
    /// Icon name of the item's type.
    pub icon: &'static str,
}

/// Special places of the tree.
pub const UNGROUPED: &str = ":ungrouped";
pub const TRASH: &str = ":trash";

/// A row of the tree.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Row {
    /// A folder (also *Ungrouped* and *Trash*, by their special ids).
    Folder { id: String, name: String, depth: usize, collapsed: bool, count: usize },
    Item { item: Item, depth: usize },
}

fn sort_key(s: &str) -> String {
    s.to_lowercase()
}

/// Sub-folders of `parent`, sorted by name.
fn children<'a>(folders: &'a [Folder], parent: Option<&str>) -> Vec<&'a Folder> {
    let mut out: Vec<&Folder> = folders.iter().filter(|f| f.parent.as_deref() == parent).collect();
    out.sort_by_key(|f| sort_key(&f.name));
    out
}

/// Notes in a folder (and all its sub-folders), without the trash.
fn count(folders: &[Folder], items: &[Item], id: &str) -> usize {
    let own = items.iter().filter(|i| !i.trashed && i.folder.as_deref() == Some(id)).count();
    own + children(folders, Some(id)).iter().map(|f| count(folders, items, &f.id)).sum::<usize>()
}

/// The rows of the tree: folders with their notes, then *Ungrouped* (notes
/// in no folder) and, if not empty, *Trash*. `collapsed` special places are
/// given by their ids.
pub fn rows(folders: &[Folder], items: &[Item], collapsed_special: &HashSet<String>, labels: (&str, &str)) -> Vec<Row> {
    let known: HashSet<&str> = folders.iter().map(|f| f.id.as_str()).collect();
    let mut out = Vec::new();
    fn items_in<'a>(items: &'a [Item], folder: Option<&str>, known: &HashSet<&str>) -> Vec<&'a Item> {
        let mut list: Vec<&Item> = items
            .iter()
            .filter(|i| !i.trashed)
            .filter(|i| match (folder, i.folder.as_deref()) {
                (Some(f), Some(g)) => f == g,
                // Notes of a folder that no longer exists are ungrouped.
                (None, g) => g.is_none_or(|g| !known.contains(g)),
                _ => false,
            })
            .collect();
        list.sort_by_key(|i| sort_key(&i.title));
        list
    }
    fn walk(out: &mut Vec<Row>, folders: &[Folder], items: &[Item], known: &HashSet<&str>, parent: Option<&str>, depth: usize) {
        for f in children(folders, parent) {
            out.push(Row::Folder {
                id: f.id.clone(),
                name: f.name.clone(),
                depth,
                collapsed: f.collapsed,
                count: count(folders, items, &f.id),
            });
            if !f.collapsed {
                walk(out, folders, items, known, Some(&f.id), depth + 1);
                for item in items_in(items, Some(&f.id), known) {
                    out.push(Row::Item { item: item.clone(), depth: depth + 1 });
                }
            }
        }
    }
    walk(&mut out, folders, items, &known, None, 0);
    let ungrouped = items_in(items, None, &known);
    let collapsed = collapsed_special.contains(UNGROUPED);
    out.push(Row::Folder { id: UNGROUPED.into(), name: labels.0.into(), depth: 0, collapsed, count: ungrouped.len() });
    if !collapsed {
        out.extend(ungrouped.into_iter().map(|i| Row::Item { item: i.clone(), depth: 1 }));
    }
    let mut trash: Vec<&Item> = items.iter().filter(|i| i.trashed).collect();
    if !trash.is_empty() {
        trash.sort_by_key(|i| sort_key(&i.title));
        let collapsed = collapsed_special.contains(TRASH);
        out.push(Row::Folder { id: TRASH.into(), name: labels.1.into(), depth: 0, collapsed, count: trash.len() });
        if !collapsed {
            out.extend(trash.into_iter().map(|i| Row::Item { item: i.clone(), depth: 1 }));
        }
    }
    out
}

/// A folder name not yet used among the sub-folders of `parent`.
pub fn unique_name(folders: &[Folder], parent: Option<&str>, wanted: &str, except: Option<&str>) -> String {
    let taken: HashSet<String> = children(folders, parent)
        .iter()
        .filter(|f| Some(f.id.as_str()) != except)
        .map(|f| sort_key(&f.name))
        .collect();
    let wanted = wanted.trim();
    let wanted = if wanted.is_empty() { "Folder" } else { wanted };
    if !taken.contains(&sort_key(wanted)) {
        return wanted.to_owned();
    }
    (2..).map(|n| format!("{wanted} {n}")).find(|n| !taken.contains(&sort_key(n))).unwrap_or_default()
}

/// The folder at the path `names` below `parent`: existing folders are
/// found by name (ignoring case), missing ones are created with `new_id`.
pub fn ensure_path(folders: &mut Vec<Folder>, parent: Option<&str>, names: &[String], mut new_id: impl FnMut() -> String) -> Option<String> {
    let mut current = parent.map(str::to_owned);
    for name in names {
        let name = name.trim();
        let name = if name.is_empty() { "Folder" } else { name };
        let found = children(folders, current.as_deref()).iter().find(|f| sort_key(&f.name) == sort_key(name)).map(|f| f.id.clone());
        let id = match found {
            Some(id) => id,
            None => {
                let id = new_id();
                folders.push(Folder { id: id.clone(), name: name.to_owned(), parent: current.clone(), collapsed: true });
                id
            }
        };
        current = Some(id);
    }
    current
}

/// Whether `folder` is `ancestor` or lies inside it.
pub fn is_inside(folders: &[Folder], folder: &str, ancestor: &str) -> bool {
    let parents: HashMap<&str, Option<&str>> = folders.iter().map(|f| (f.id.as_str(), f.parent.as_deref())).collect();
    let mut cur = Some(folder);
    let mut steps = 0;
    while let Some(id) = cur {
        if id == ancestor {
            return true;
        }
        steps += 1;
        if steps > folders.len() + 1 {
            return false;
        }
        cur = parents.get(id).copied().flatten();
    }
    false
}

/// Moves a folder into `target` (`None`: top level). Refuses moves into
/// itself or its own sub-folders.
pub fn move_folder(folders: &mut [Folder], id: &str, target: Option<&str>) -> bool {
    if target.is_some_and(|t| is_inside(folders, t, id)) {
        return false;
    }
    let name = match folders.iter().find(|f| f.id == id) {
        Some(f) if f.parent.as_deref() != target => f.name.clone(),
        _ => return false,
    };
    let name = unique_name(folders, target, &name, Some(id));
    if let Some(f) = folders.iter_mut().find(|f| f.id == id) {
        f.parent = target.map(str::to_owned);
        f.name = name;
    }
    true
}

/// Removes a folder; its sub-folders move to its parent. Returns the
/// parent, where its notes go.
pub fn remove_folder(folders: &mut Vec<Folder>, id: &str) -> Option<Option<String>> {
    let pos = folders.iter().position(|f| f.id == id)?;
    let parent = folders.remove(pos).parent;
    let moved: Vec<String> = folders.iter().filter(|f| f.parent.as_deref() == Some(id)).map(|f| f.id.clone()).collect();
    for child in moved {
        // Keeps names unique among the new siblings.
        move_folder(folders, &child, parent.as_deref());
    }
    Some(parent)
}

/// The path of a folder, e.g. "Work / Project X".
pub fn path(folders: &[Folder], id: &str) -> String {
    let mut names = Vec::new();
    let mut cur = folders.iter().find(|f| f.id == id);
    while let Some(f) = cur {
        names.push(f.name.as_str());
        if names.len() > folders.len() {
            break;
        }
        cur = f.parent.as_deref().and_then(|p| folders.iter().find(|g| g.id == p));
    }
    names.reverse();
    names.join(" / ")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ensure_path_finds_and_creates() {
        let mut folders = vec![Folder { id: "v".into(), name: "Vault".into(), parent: None, collapsed: false }, Folder {
            id: "w".into(),
            name: "Work".into(),
            parent: Some("v".into()),
            collapsed: false,
        }];
        let mut n = 0;
        let mut next = || {
            n += 1;
            format!("new{n}")
        };
        let names = ["work".to_owned(), "Deep".to_owned()];
        assert_eq!(ensure_path(&mut folders, Some("v"), &names, &mut next).as_deref(), Some("new1"));
        assert_eq!(ensure_path(&mut folders, Some("v"), &names, &mut next).as_deref(), Some("new1"));
        assert_eq!(folders.len(), 3);
        assert_eq!(folders[2].parent.as_deref(), Some("w"));
        assert_eq!(ensure_path(&mut folders, Some("v"), &[], &mut next).as_deref(), Some("v"));
    }

    fn folder(id: &str, name: &str, parent: Option<&str>) -> Folder {
        Folder { id: id.into(), name: name.into(), parent: parent.map(str::to_owned), collapsed: false }
    }

    fn item(id: &str, folder: Option<&str>) -> Item {
        Item {
            id: id.into(),
            title: id.to_uppercase(),
            folder: folder.map(str::to_owned),
            trashed: false,
            open: false,
            file: false,
            missing: false,
            icon: "",
        }
    }

    fn shape(rows: &[Row]) -> Vec<String> {
        rows.iter()
            .map(|r| match r {
                Row::Folder { id, depth, count, .. } => format!("{}{id}({count})", "  ".repeat(*depth)),
                Row::Item { item, depth } => format!("{}{}", "  ".repeat(*depth), item.id),
            })
            .collect()
    }

    #[test]
    fn builds_the_tree() {
        let folders = vec![folder("w", "Work", None), folder("p", "Project", Some("w")), folder("a", "Archive", None)];
        let mut items = vec![item("plan", Some("p")), item("meet", Some("w")), item("loose", None), item("lost", Some("gone"))];
        let mut gone = item("old", Some("w"));
        gone.trashed = true;
        items.push(gone);
        let rows = rows(&folders, &items, &HashSet::new(), ("Ungrouped", "Trash"));
        assert_eq!(
            shape(&rows),
            [
                "a(0)", "w(2)", "  p(1)", "    plan", "  meet",
                ":ungrouped(2)", "  loose", "  lost", ":trash(1)", "  old",
            ]
        );
        // Collapsed folders hide their contents.
        let mut folded = folders.clone();
        folded[0].collapsed = true;
        let collapsed: HashSet<String> = [UNGROUPED.to_owned()].into();
        let rows = super::rows(&folded, &items, &collapsed, ("U", "T"));
        assert_eq!(shape(&rows), ["a(0)", "w(2)", ":ungrouped(2)", ":trash(1)", "  old"]);
    }

    #[test]
    fn folders_move_without_cycles() {
        let mut folders = vec![folder("w", "Work", None), folder("p", "Project", Some("w")), folder("x", "Project", None)];
        assert!(!move_folder(&mut folders, "w", Some("p")));
        assert!(!move_folder(&mut folders, "w", Some("w")));
        assert!(move_folder(&mut folders, "x", Some("w")));
        let x = folders.iter().find(|f| f.id == "x").unwrap();
        assert_eq!((x.parent.as_deref(), x.name.as_str()), (Some("w"), "Project 2"));
        assert_eq!(path(&folders, "x"), "Work / Project 2");
        assert!(is_inside(&folders, "x", "w"));
    }

    #[test]
    fn removing_a_folder_keeps_its_contents() {
        let mut folders = vec![folder("w", "Work", None), folder("p", "Project", Some("w")), folder("q", "Q", Some("p"))];
        assert_eq!(remove_folder(&mut folders, "p"), Some(Some("w".into())));
        assert_eq!(folders.iter().find(|f| f.id == "q").unwrap().parent.as_deref(), Some("w"));
        assert_eq!(remove_folder(&mut folders, "nope"), None);
    }

    #[test]
    fn names_and_paths() {
        let mut folders = vec![folder("w", "Work", None)];
        assert_eq!(unique_name(&folders, None, "work", None), "work 2");
        assert_eq!(unique_name(&folders, None, "Work", Some("w")), "Work");
        assert_eq!(unique_name(&folders, Some("w"), "  ", None), "Folder");
        folders.push(folder("a", "A", Some("w")));
        assert_eq!(path(&folders, "a"), "Work / A");
    }
}
