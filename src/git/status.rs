use crate::models::path_items::PathItems;
use git2::{Repository, Status};
use std::process;

/// Gets the file paths of the changes in your repo.
pub fn get_paths(repo: &Repository) -> Result<Vec<PathItems>, git2::Error> {
    let statuses = repo.statuses(None)?;

    if statuses.is_empty() {
        println!("{}", console::style("✔ working tree clean ✔").green());
        return Ok(vec![]);
    }

    let mut items: Vec<PathItems> = vec![];

    for diff_entry in statuses.iter() {
        if diff_entry.status() == Status::IGNORED {
            continue;
        }

        let path_items = diff_entry
            // 1. Try to get the HEAD → index diff
            .head_to_index()
            // If the file differs between HEAD and Index, grab the new file path. (This means the file has been staged.)
            .and_then(|d| {
                Some(PathItems {
                    path: String::from(d.new_file().path()?.display().to_string()),
                    is_staged: true,
                    is_selected: false,
                })
            })
            // 2. Otherwise, try index → workdir diff (This means the file has unstaged changes.)
            .or_else(|| {
                Option::from(
                    diff_entry
                        .index_to_workdir()
                        .and_then(|d| {
                            Some(PathItems {
                                path: String::from(d.new_file().path()?.display().to_string()),
                                is_staged: false,
                                is_selected: false,
                            })
                        })
                        // 3. If still nothing, try the "old" file's path (maybe a deletion/rename)
                        // If the file is gone in workdir (deleted) or renamed, take the old file path
                        .or_else(|| {
                            diff_entry.index_to_workdir().and_then(|d| {
                                Some(PathItems {
                                    path: String::from(d.old_file().path()?.display().to_string()),
                                    is_staged: false,
                                    is_selected: false,
                                })
                            })
                        })
                        // 4. If nothing worked, fallback to "<unknown>"
                        .unwrap_or_else(|| PathItems {
                            path: String::from("<unknown>"),
                            is_staged: false,
                            is_selected: false,
                        }),
                )
            })
            .unwrap();

        items.push(path_items);
    }

    // If the only changes are ignored files, exit
    if items.is_empty() {
        println!("{}", console::style("✔ working tree clean ✔").green());
        process::exit(1)
    }

    Ok(items)
}

#[cfg(test)]
mod tests {
    use super::*;
    use git2::{Oid, Repository, Signature};
    use std::fs::File;
    use std::io::Write;
    use std::path::Path;
    use tempfile::TempDir;

    /// Helper to initialize a new git repository in a temp dir
    fn init_repo() -> (TempDir, Repository) {
        let tmp_dir = TempDir::new().expect("create temp dir");
        let repo = Repository::init(tmp_dir.path()).expect("init repo");
        (tmp_dir, repo)
    }

    /// Helper to commit a file to the repo
    fn commit_file(repo: &Repository, file_path: &str, content: &str, message: &str) -> Oid {
        let mut file = File::create(repo.workdir().unwrap().join(file_path)).unwrap();
        file.write_all(content.as_bytes()).unwrap();

        let mut index = repo.index().unwrap();
        index.add_path(Path::new(file_path)).unwrap();
        let oid = index.write_tree().unwrap();

        let sig = Signature::now("Test", "test@example.com").unwrap();
        let tree = repo.find_tree(oid).unwrap();

        let parent_commit = repo
            .head()
            .ok()
            .and_then(|h| h.target())
            .and_then(|oid| repo.find_commit(oid).ok());

        let commit_oid = if let Some(parent) = parent_commit {
            repo.commit(Some("HEAD"), &sig, &sig, message, &tree, &[&parent])
                .unwrap()
        } else {
            repo.commit(Some("HEAD"), &sig, &sig, message, &tree, &[])
                .unwrap()
        };
        commit_oid
    }

    #[test]
    fn test_get_paths_empty_worktree() {
        let (_tmp, repo) = init_repo();

        // No files, clean worktree
        let statuses = get_paths(&repo).unwrap();
        assert!(statuses.is_empty());
    }

    #[test]
    fn test_get_paths_unstaged_file() {
        let (_tmp, repo) = init_repo();

        // Create a file but do not stage it
        let file_path = "foo.txt";
        let file_full_path = repo.workdir().unwrap().join(file_path);
        let mut file = File::create(&file_full_path).unwrap();
        writeln!(file, "hello world").unwrap();

        // Now, get_paths should return one PathItems with is_staged == false
        let paths = get_paths(&repo).unwrap();
        assert_eq!(paths.len(), 1);
        let item = &paths[0];
        assert_eq!(item.path, file_path);
        assert!(!item.is_staged);
        assert!(!item.is_selected);
    }

    #[test]
    fn test_get_paths_staged_file() {
        let (_tmp, repo) = init_repo();

        // Create and stage a file
        let file_path = "bar.txt";
        let file_full_path = repo.workdir().unwrap().join(file_path);
        let mut file = File::create(&file_full_path).unwrap();
        writeln!(file, "hello staged").unwrap();

        let mut index = repo.index().unwrap();
        index.add_path(Path::new(file_path)).unwrap();
        index.write().unwrap();

        // Now, get_paths should return one PathItems with is_staged == true
        let paths = get_paths(&repo).unwrap();
        assert_eq!(paths.len(), 1);
        let item = &paths[0];
        assert_eq!(item.path, file_path);
        assert!(item.is_staged);
        assert!(!item.is_selected);
    }

    #[test]
    fn test_get_paths_staged_and_unstaged() {
        let (_tmp, repo) = init_repo();

        // Commit an initial file
        commit_file(&repo, "init.txt", "init", "init commit");

        // Add and stage a file
        let staged_path = "staged.txt";
        let staged_full_path = repo.workdir().unwrap().join(staged_path);
        let mut staged_file = File::create(&staged_full_path).unwrap();
        writeln!(staged_file, "staged content").unwrap();

        let mut index = repo.index().unwrap();
        index.add_path(Path::new(staged_path)).unwrap();
        index.write().unwrap();

        // Add an unstaged file
        let unstaged_path = "unstaged.txt";
        let unstaged_full_path = repo.workdir().unwrap().join(unstaged_path);
        let mut unstaged_file = File::create(&unstaged_full_path).unwrap();
        writeln!(unstaged_file, "unstaged content").unwrap();

        // Now, get_paths should return two PathItems
        let mut paths = get_paths(&repo).unwrap();
        paths.sort_by(|a, b| a.path.cmp(&b.path));
        assert_eq!(paths.len(), 2);

        let staged = paths.iter().find(|p| p.path == staged_path).unwrap();
        assert!(staged.is_staged);

        let unstaged = paths.iter().find(|p| p.path == unstaged_path).unwrap();
        assert!(!unstaged.is_staged);
    }
}
