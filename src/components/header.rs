// 标题栏组件
use crate::framework::*;
use ratatui::{prelude::*, widgets::*};
use anyhow::Result;

pub struct Header {
    pub message: String,
}

impl Header {
    pub fn new(msg: &str) -> Self {
        Self { message: msg.to_string() }
    }
}

impl Component for Header {
    fn handle_event(&mut self, event: &AppEvent) -> Result<Option<CustomAction>> {
        if let AppEvent::Action(CustomAction::Notify(msg)) = event {
            self.message = msg.clone();
        }
        Ok(None)
    }

    fn render(&self, frame: &mut Frame, area: Rect) {
        let p = Paragraph::new(format!(" 状态通知: {}", self.message))
            .block(Block::bordered().title(" Morphic TUI v0.1.0 ")) // 0.30 推荐写法
            .style(Style::default().fg(Color::Cyan));
        frame.render_widget(p, area);
    }
}