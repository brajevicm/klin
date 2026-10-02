pub mod db;
pub mod export;
pub mod http;
pub mod money;
pub mod notify;
pub mod payments;
pub mod shared;
pub mod ui;

pub type Result<T> = std::result::Result<T, String>;
