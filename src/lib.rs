use std::process;
use dialoguer::MultiSelect;
use git2::{Repository, Status};

struct PathItems {
    items: Vec<String>,
    default_checked: Vec<bool>
}

/// Gets the file paths of the changes in your repo.
fn get_paths() -> Result<PathItems, git2::Error> {
    let repo = Repository::open(".")?;
    let statuses = repo.statuses(None)?;

    if statuses.is_empty() {
        println!("{}", console::style("✔ working tree clean").green());
        process::exit(1)
    }

    let mut items = vec![];
    let mut default_checked = vec![];

    for diff_entry in statuses.iter() {
        let path = diff_entry
            // 1. Try to get the HEAD → index diff
            .head_to_index()
            // If the file differs between HEAD and Index, grab the new file path. (This means the file has been staged.)
            .and_then(|d| d.new_file().path())
            // 2. Otherwise, try index → workdir diff (This means the file has unstaged changes.)
            .or_else(|| diff_entry.index_to_workdir().and_then(|d| d.new_file().path()))
            // 3. If still nothing, try the "old" file's path (maybe a deletion/rename)
            // If the file is gone in workdir (deleted) or renamed, take the old file path
            .or_else(|| diff_entry.index_to_workdir().and_then(|d| d.old_file().path()))
            // 4. If we found a path, display it as a String
            .map(|p| p.display().to_string())
            // 5. If nothing worked, fallback to "<unknown>"
            .unwrap_or_else(|| "<unknown>".into());

        let status = diff_entry.status();

        let is_staged = is_staged(status);

        if status.contains(Status::IGNORED) {
            continue;
        }
        items.push(path);
        default_checked.push(is_staged);
    }

    Ok(PathItems {
        items,
        default_checked,
    })
}

pub fn choose_files() -> Vec<String> {
    let path_items = get_paths().unwrap_or_else(|_| {
        println!("{}", console::style("Not a git repository").red());
        process::exit(1)
    });

    if path_items.items.is_empty() {
        println!("{}", console::style("✔ working tree clean ✔").green());
        process::exit(1);
    }

    let selections = MultiSelect::new()
        .with_prompt("Choose files to stage")
        .items(&path_items.items)
        .defaults(&path_items.default_checked)
        .interact()
        .unwrap_or_else(|_| {
            println!("{}", console::style("No files selected").red());
            process::exit(1)
        });

    let mut paths: Vec<String> = vec![];

    for selected in selections {
        println!("{}", path_items.items[selected]);
        paths.push(path_items.items[selected].clone());
    }

    paths
}

fn is_staged(s: Status) -> bool {
    // Index (staged)
    // If it is staged, it should be pre-selected when running ccb

    let index = vec![
        Status::INDEX_NEW,
        Status::INDEX_MODIFIED,
        Status::INDEX_DELETED,
        Status::INDEX_RENAMED,
        Status::INDEX_TYPECHANGE,
        Status::CONFLICTED,
    ];

    for status in index {
        if s.contains(status) {
            return true;
        }
    }

    if s.contains(Status::WT_NEW) && s.contains(Status::INDEX_NEW) {
        true
    } else if s.contains(Status::WT_MODIFIED) && s.contains(Status::INDEX_MODIFIED) {
        return true;
    } else if s.contains(Status::WT_DELETED) && s.contains(Status::INDEX_DELETED) {
        return true;
    } else if s.contains(Status::WT_RENAMED) && s.contains(Status::INDEX_RENAMED) {
        return true;
    } else if s.contains(Status::WT_TYPECHANGE) && s.contains(Status::INDEX_TYPECHANGE) {
        return true;
    } else if s.contains(Status::WT_NEW) && !s.intersects(Status::INDEX_NEW) {
        return true;
    } else {
        let worktree = vec![
            Status::WT_NEW,
            Status::WT_MODIFIED,
            Status::WT_DELETED,
            Status::WT_RENAMED,
            Status::WT_TYPECHANGE,
            Status::CONFLICTED
        ];

        for status in worktree {
            if s.contains(status) {
                return false;
            }
        }

        return false;
    }
}

pub fn git_add_selected(repo: &Repository, paths: &Vec<String>) {
    let mut index = repo.index().unwrap();
    index.add_all(paths, git2::IndexAddOption::DEFAULT, None).unwrap();
    index.write().unwrap();
}

// Tests

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_is_staged() {
        assert!(is_staged(Status::INDEX_NEW));
        assert!(is_staged(Status::INDEX_MODIFIED));
        assert!(is_staged(Status::INDEX_DELETED));
        assert!(is_staged(Status::INDEX_RENAMED));
        assert!(is_staged(Status::INDEX_TYPECHANGE));
        assert!(is_staged(Status::CONFLICTED));

        assert!(is_staged(Status::WT_NEW | Status::INDEX_NEW));
        assert!(is_staged(Status::WT_MODIFIED | Status::INDEX_MODIFIED));
        assert!(is_staged(Status::WT_DELETED | Status::INDEX_DELETED));
        assert!(is_staged(Status::WT_RENAMED | Status::INDEX_RENAMED));
        assert!(is_staged(Status::WT_TYPECHANGE | Status::INDEX_TYPECHANGE));

        assert!(is_staged(Status::WT_NEW));
        assert!(!is_staged(Status::WT_MODIFIED));
        assert!(!is_staged(Status::WT_DELETED));
        assert!(!is_staged(Status::WT_RENAMED));
        assert!(!is_staged(Status::WT_TYPECHANGE));

        assert!(is_staged(Status::WT_NEW | Status::INDEX_MODIFIED));
        assert!(is_staged(Status::WT_MODIFIED | Status::INDEX_DELETED));
        assert!(is_staged(Status::WT_DELETED | Status::INDEX_RENAMED));
        assert!(is_staged(Status::WT_RENAMED | Status::INDEX_TYPECHANGE));
        assert!(is_staged(Status::WT_TYPECHANGE | Status::INDEX_NEW));

        assert!(!is_staged(Status::empty()));
    }

    #[test]
    fn test_get_paths_no_repo() {
        let result = get_paths();
        assert!(result.is_err());
    }
}
