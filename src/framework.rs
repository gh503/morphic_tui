// 核心 Trait 与事件定义
use anyhow::Result;
use ratatui::prelude::*;
use crate::config::AppConfig;

#[derive(Debug, Clone)]
pub enum CustomAction {
    UpdateCpu(f32),
    UpdateMemory { used: u64, total: u64 }, // 新增：内存数据
    ToggleSidebar,
    NextApp,
    SetHistory(usize), // 新增：动态调整历史点数
    ResizeSidebar(u16), // 新增：设置侧边栏绝对宽度

    RefreshData,          // 意图：发起刷新
    SyncDatabaseFinished, // 意图：刷新完成信号
    SaveConfig,
    NotifySuccess(String),        // 意图：操作成功，要求系统弹出提示（Toast）
    NotifyError(String),          // 意图：操作失败
    UpdateTaskStatus { id: String, status: String }, // 意图：更新任务状态
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