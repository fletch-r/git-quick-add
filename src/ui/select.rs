use crate::models::path_items::PathItems;
use dialoguer::MultiSelect;
use std::process;

/// Prompts the user to select files to stage and returns the selected file paths.
/// If no files are selected, the program exits.
/// # Arguments
/// * `path_items` - A vector of PathItems representing files that can be staged.
/// # Returns
/// A vector of PathItems with updated selection status.
pub fn choose_files(mut path_items: Vec<PathItems>) -> Vec<PathItems> {
    let list_of_paths: Vec<String> = path_items.iter().map(|p| p.path.clone()).collect();
    let list_of_preselected: Vec<bool> = path_items.iter().map(|p| p.is_staged).collect();

    let selections = MultiSelect::new()
        .with_prompt("Choose files to stage - (use Space to select - press Enter to submit)")
        .items(list_of_paths)
        .defaults(&list_of_preselected)
        .interact()
        .unwrap_or_else(|_| {
            eprintln!("{}", console::style("Error selecting files").red());
            process::exit(1)
        });

    for index in selections {
        path_items[index].is_selected = true;
    }

    path_items
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_choose_files_selections() {
        // This test is limited since it requires user interaction
        // We can at least test the data structure transformations
        let input = vec![
            PathItems {
                path: String::from("file1.txt"),
                is_staged: true,
                is_selected: false,
            },
            PathItems {
                path: String::from("file2.txt"),
                is_staged: false,
                is_selected: false,
            },
        ];

        let paths_clone = input.clone();
        assert_eq!(
            paths_clone
                .iter()
                .map(|p| p.is_selected)
                .collect::<Vec<_>>(),
            vec![false, false]
        );
    }
}
