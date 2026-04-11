// src/framework.rs
use anyhow::Result;
use ratatui::prelude::*;
use crate::config::AppConfig;

#[derive(Debug, Clone, PartialEq)]
pub enum CustomAction {
    UpdateCpu(f32),
    UpdateMemory { used: u64, total: u64 },
    ToggleSidebar,
    NextApp,
    SetHistory(usize),
    ResizeSidebar(u16),
    RefreshData,
    SyncDatabaseFinished,
    SaveConfig,
    NotifySuccess(String),
    NotifyError(String),
}

pub enum AppEvent {
    Tick,
    Key(crossterm::event::KeyEvent),
    Mouse(crossterm::event::MouseEvent),
    Action(CustomAction), 
}

pub trait Component {
    fn handle_event(&mut self, event: &AppEvent) -> Result<Option<CustomAction>>;
    fn render(&self, frame: &mut Frame, area: Rect, config: &AppConfig);
}