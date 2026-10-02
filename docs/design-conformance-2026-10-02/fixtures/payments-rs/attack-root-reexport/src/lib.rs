pub mod db;
pub mod export;
pub mod http;
pub mod money;
pub mod notify;
pub mod payments;
pub mod ui;

pub type Result<T> = std::result::Result<T, String>;

pub use ui::labels::status_label;
