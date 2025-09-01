use std::process;
use dialoguer::MultiSelect;
use git2::{Repository, Status};

#[derive(Clone)]
pub struct PathItems {
    path:      String,
    is_staged: bool,
    is_selected: bool,
}

impl Default for PathItems {
    fn default() -> Self {
        PathItems {
            path: String::new(),
            is_staged: false,
            is_selected: false,
        }
    }
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
            println!("{}", console::style("No files selected").red());
            process::exit(1)
        });

    let mut paths: Vec<PathItems> = path_items.clone();

    for index in selections {
        paths[index].is_selected = true;
    }

    paths
}

// Step 4
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

            // println!(" - {} {}", console::style("Unstaged:").yellow(), item.path.clone());
            logs.push(format!(" - {} {}", console::style("Unstaged:").yellow(), item.path.clone()));
        } else if !item.is_staged && item.is_selected {
            let list_of_paths: Vec<String> = paths.iter()
                                                .filter(|x| !x.is_staged && x.is_selected)
                                                .map(|p| p.path.clone()).collect();

            index.add_all(&list_of_paths, git2::IndexAddOption::DEFAULT, None)?;
            list_of_paths.iter().for_each(|p| {
                // println!(" - {} {}", console::style("Staged:").green(), p);
                logs.push(format!(" - {} {}", console::style("Staged:").green(), p));
            });
            index.write().unwrap_or_else(|_| {
                println!("{}", console::style("Failed to write index").red());
                process::exit(1)
            });
        } else {
            if item.is_staged {
                // println!(" - {} {}", console::style("Staged:").green(), item.path.clone());
                logs.push(format!(" - {} {}", console::style("Staged:").green(), item.path.clone()));
            } else {
                // println!(" - {} {}", console::style("Unstaged:").yellow(), item.path.clone());
                logs.push(format!(" - {} {}", console::style("Unstaged:").yellow(), item.path.clone()));
            }
        }
    }

    println!("{}", logs.join("\n"));

    Ok(())
}

// Step 1
/// Gets the file paths of the changes in your repo.
pub fn get_paths(repo: &Repository) -> Result<Vec<PathItems>, git2::Error> {
    let statuses = repo.statuses(None)?;

    if statuses.is_empty() {
        println!("{}", console::style("✔ working tree clean ✔").green());
        process::exit(1)
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
            .and_then(|d| Some(PathItems {
                path: String::from(d.new_file().path()?.display().to_string()),
                is_staged: true,
                is_selected: false
            }))
            // 2. Otherwise, try index → workdir diff (This means the file has unstaged changes.)
            .or_else(|| Option::from(diff_entry.index_to_workdir().and_then(|d| Some(PathItems {
                    path: String::from(d.new_file().path()?.display().to_string()),
                    is_staged: false,
                    is_selected: false
                }))
                // 3. If still nothing, try the "old" file's path (maybe a deletion/rename)
                // If the file is gone in workdir (deleted) or renamed, take the old file path
                .or_else(|| diff_entry.index_to_workdir().and_then(|d| Some(PathItems {
                    path: String::from(d.old_file().path()?.display().to_string()),
                    is_staged: false,
                    is_selected: false
                })))
                // 4. If nothing worked, fallback to "<unknown>"
                .unwrap_or_else(|| PathItems {
                    path: String::from("<unknown>"),
                    is_staged: false,
                    is_selected: false
                }))).unwrap();

        items.push(path_items);
    }

    // If the only changes are ignored files, exit
    if items.is_empty() {
        println!("{}", console::style("✔ working tree clean ✔").green());
        process::exit(1)
    }

    Ok(items)
}

// Tests
#[cfg(test)]
mod tests {

}