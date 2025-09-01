use git2::Repository;
use git_quick_add::{choose_files, git_add_selected};

fn main() {
    let repo = Repository::open(".").unwrap_or_else(|_| panic!("Could not open repository"));

    let paths = choose_files(&repo);

    git_add_selected(&repo, &paths);
}
