pub mod debug;
pub mod panels;
pub mod windows;

pub use debug::show_debug_panel;
pub use panels::{show_central_panel, show_left_panel, show_search_panel};
pub use windows::{WindowActions, WindowManager};
