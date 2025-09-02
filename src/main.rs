use std::process;
use git2::Repository;
use git_quick_add::{choose_files, get_paths, git_add_selected};

fn main() {
    let repo = Repository::open(".").unwrap_or_else(|_| {
        eprintln!("{}", console::style("Not a git repository").red());
        process::exit(1)
    });

    let paths = get_paths(&repo).unwrap_or_else(|_| {
        eprintln!("{}", console::style("No files found").red());
        process::exit(1)
    });

    let chosen = choose_files(paths);

    git_add_selected(&repo, &chosen).unwrap_or_else(|_| {
        eprintln!("{}", console::style("Failed to stage files").red());
        process::exit(1)
    });
}
