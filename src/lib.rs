mod app;
mod calc;
mod colors;
mod init;

pub use app::DaytimePopulationApp;
pub use colors::MfColors;
use once_cell::sync::OnceCell;

pub static POPULATION_COUNT: OnceCell<Vec<u64>> = OnceCell::new();
