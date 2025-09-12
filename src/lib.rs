pub mod git {
    mod stage;
    mod status;

    pub use stage::git_add_selected;
    pub use status::get_paths;
}

pub mod models {
    pub mod path_items;
    pub use path_items::PathItems;
}

pub mod ui {
    mod select;
    pub use select::choose_files;
}

// Re-export commonly used items at the crate root
pub use git::{get_paths, git_add_selected};
pub use models::PathItems;
pub use ui::choose_files;
