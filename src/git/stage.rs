use crate::models::path_items::PathItems;
use git2::Repository;
use std::path::Path;

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
        // if the item is not staged and selected, we need to stage it
        } else if !item.is_staged && item.is_selected {
            let p = Path::new(&item.path);

            index.add_path(p).unwrap_or_else(|error| {
                eprintln!(
                    "{} {} - {}",
                    console::style("Failed to add path for").red(),
                    console::style(&item.path).yellow(),
                    error
                );
            });

            index.write().unwrap_or_else(|error| {
                eprintln!(
                    "{} {} - {}",
                    console::style("Failed to write index for").red(),
                    console::style(&item.path).yellow(),
                    error
                );
            });

            logs.push(format!(
                " - {} {}",
                console::style("Staged:").green(),
                item.path
            ));
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

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs::File;
    use std::io::Write;
    use tempfile::TempDir;

    /// Helper to initialize a new git repository in a temp dir
    fn init_repo() -> (TempDir, Repository) {
        let tmp_dir = TempDir::new().expect("create temp dir");
        let repo = Repository::init(tmp_dir.path()).expect("init repo");
        (tmp_dir, repo)
    }

    #[test]
    fn test_git_add_selected() {
        let (_tmp, repo) = init_repo();

        // Create a test file
        let file_path = "test.txt";
        let file_full_path = repo.workdir().unwrap().join(file_path);
        let mut file = File::create(&file_full_path).unwrap();
        writeln!(file, "test content").unwrap();

        let paths = vec![PathItems {
            path: String::from(file_path),
            is_staged: false,
            is_selected: true,
        }];

        git_add_selected(&repo, &paths).unwrap();

        // Verify the file is staged
        let statuses = repo.statuses(None).unwrap();
        for status in statuses.iter() {
            assert!(status.status().is_index_new());
        }
    }
}
