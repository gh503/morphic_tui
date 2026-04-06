use serde::{Serialize, Deserialize};

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct AppConfig {
    pub sidebar_width: u16,
    pub max_points: usize,
    pub show_sidebar: bool,
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            sidebar_width: 25,
            max_points: 50,
            show_sidebar: true,
        }
    }
}