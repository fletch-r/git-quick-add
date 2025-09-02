use dialoguer::MultiSelect;
use git2::{Repository, Status};
use std::{path::Path, process};

#[derive(Clone, Debug)]
pub struct PathItems {
    path: String,
    is_staged: bool,
    is_selected: bool,
}

// Step 1
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

// Step 2
/// Prompts the user to select files to stage and returns the selected file paths.
/// If no files are selected, the program exits.
/// # Arguments
/// * `repo` - A reference to the git repository.
/// # Returns
/// A vector of selected file paths as strings.
pub fn choose_files(path_items: Vec<PathItems>) -> Vec<PathItems> {
    // TODO: Include the status of each file in the prompt (e.g., "M", "A", "D", "??")
    let list_of_paths: Vec<String> = path_items.iter().map(|p| p.path.clone()).collect();
    let list_of_preselected: Vec<bool> = path_items.iter().map(|p| p.is_staged).collect();

    let selections = MultiSelect::new()
        .with_prompt("Choose files to stage")
        .items(list_of_paths)
        .defaults(&list_of_preselected)
        .interact()
        .unwrap_or_else(|_| {
            eprintln!("{}", console::style("Error selecting files").red());
            process::exit(1)
        });

    let mut paths: Vec<PathItems> = path_items.clone();

    for index in selections {
        paths[index].is_selected = true;
    }

    paths
}

// Step 3
/// Stages the selected files in the git repository.
/// If staging fails, the program exits.
/// # Arguments
/// * `repo` - A reference to the git repository.
/// * `paths` - A vector of file paths to stage.
pub fn git_add_selected(repo: &Repository, paths: &Vec<PathItems>) -> Result<(), git2::Error> {
    let mut index = repo.index()?;

    println!("{}", console::style("Changes Made:").bold());

    let mut logs = vec![];

    for item in paths {
        // if the item is staged and not selected, we need to unstage it
        if item.is_staged && !item.is_selected {
            let target = repo.head()?.peel(git2::ObjectType::Commit)?;
            repo.reset_default(Some(&target), &[&item.path])?;

            logs.push(format!(
                " - {} {}",
                console::style("Unstaged:").yellow(),
                item.path.clone()
            ));
        } else if !item.is_staged && item.is_selected {
            let p = Path::new(&item.path);

            index.add_path(p).unwrap_or_else(|e| {
                eprintln!("{}", e);
                process::exit(1)
            });

            logs.push(format!(
                " - {} {}",
                console::style("Staged:").green(),
                item.path
            ));

            index.write().unwrap_or_else(|_| {
                println!("{}", console::style("Failed to write index").red());
                process::exit(1)
            });
        } else {
            if item.is_staged {
                logs.push(format!(
                    " - {} {}",
                    console::style("Staged:").green(),
                    item.path.clone()
                ));
            } else {
                logs.push(format!(
                    " - {} {}",
                    console::style("Unstaged:").yellow(),
                    item.path.clone()
                ));
            }
        }
    }

    println!("{}", logs.join("\n"));

    Ok(())
}

// Tests
#[cfg(test)]
mod tests {
use super::*;
use git2::{Repository, Signature, Oid};
use std::fs::File;
use std::io::Write;
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

    let parent_commit = repo.head().ok()
        .and_then(|h| h.target())
        .and_then(|oid| repo.find_commit(oid).ok());

    let commit_oid = if let Some(parent) = parent_commit {
        repo.commit(
            Some("HEAD"),
            &sig,
            &sig,
            message,
            &tree,
            &[&parent],
        ).unwrap()
    } else {
        repo.commit(
            Some("HEAD"),
            &sig,
            &sig,
            message,
            &tree,
            &[],
        ).unwrap()
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
