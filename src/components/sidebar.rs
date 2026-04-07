// 圆角边框和基础状态显示
use crate::framework::*;
use ratatui::{prelude::*, widgets::*};
use crate::app::ActiveApp;

pub struct Sidebar {
    // 建议保留，但仅作为 handle_event 的备用存储，主要渲染将依赖参数传入
    pub cpu_last: f32,
}

impl Sidebar {
    pub const MENU_START_ROW: u16 = 4; 
    
    pub fn new() -> Self {
        Self { cpu_last: 0.0 }
    }

    /// 核心渲染方法：现在强制使用传入的 cpu_usage 参数
    pub fn render_with_state(
        &self, 
        frame: &mut Frame, 
        area: Rect, 
        active_tab: &ActiveApp, 
        cpu_usage: f32 // 关键：这是来自 MonitorApp 的实时数据
    ) {
        // 使用传入的最新值，而不是 self.cpu_last
        let display_cpu = cpu_usage; 
        let cpu_color = if display_cpu > 80.0 { Color::Red } else { Color::Cyan };
        
        // 1. 基础外框
        let block = Block::bordered()
            .title(" Morphic TUI ")
            .border_type(BorderType::Rounded)
            .border_style(Style::default().fg(cpu_color));

        // 2. 菜单项样式逻辑
        let monitor_style = if *active_tab == ActiveApp::Monitor {
            Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(Color::Gray)
        };

        let settings_style = if *active_tab == ActiveApp::Settings {
            Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(Color::Gray)
        };

        let info_style = if *active_tab == ActiveApp::Info {
            Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(Color::Gray)
        };

        let quality_style = if *active_tab == ActiveApp::Quality {
            Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(Color::Gray)
        };

        let monitor_prefix = if *active_tab == ActiveApp::Monitor { "> " } else { "  " };
        let settings_prefix = if *active_tab == ActiveApp::Settings { "> " } else { "  " };
        let info_prefix = if *active_tab == ActiveApp::Info { "> " } else { "  " };
        let quality_prefix = if *active_tab == ActiveApp::Quality { "> " } else { "  " };

        // 3. 构建 UI 文本
        let mut text = vec![
            // 实时状态区：使用 display_cpu
            Line::from(vec![
                Span::raw(" CPU: "),
                Span::styled(format!("{:.1}%", display_cpu), Style::default().fg(cpu_color).bold()),
            ]),
        ];

        // 动态生成分割线，防止宽度溢出
        let line_width = (area.width as usize).saturating_sub(2);
        text.push(Line::from("─".repeat(line_width)));

        // 导航菜单区
        text.push(Line::from(""));
        text.push(Line::from(vec![
            Span::styled(monitor_prefix, monitor_style),
            Span::styled("Monitor", monitor_style),
        ]));
        text.push(Line::from(vec![
            Span::styled(settings_prefix, settings_style),
            Span::styled("Settings", settings_style),
        ]));
        text.push(Line::from(vec![
            Span::styled(info_prefix, info_style),
            Span::styled("Info", info_style),
        ]));
        text.push(Line::from(vec![
            Span::styled(quality_prefix, quality_style),
            Span::styled("Quality", quality_style),
        ]));
        
        // 底部帮助区
        text.push(Line::from(""));
        text.push(Line::from(Span::styled(" [快捷键] ", Style::default().bg(Color::Rgb(50,50,50)))));
        text.push(Line::from(Span::styled(" Tab: 切换", Style::default().add_modifier(Modifier::DIM))));
        text.push(Line::from(Span::styled(" B:   隐藏", Style::default().add_modifier(Modifier::DIM))));
        text.push(Line::from(Span::styled(" Q:   退出", Style::default().add_modifier(Modifier::DIM))));

        let p = Paragraph::new(text).block(block);
        frame.render_widget(p, area);
    }
}

impl Component for Sidebar {
    fn handle_event(&mut self, event: &AppEvent) -> anyhow::Result<Option<CustomAction>> {
        // 即使 RootApp 统一管理了采样，保留这个 handle_event 也是好的防御性编程
        if let AppEvent::Action(CustomAction::UpdateCpu(val)) = event {
            self.cpu_last = *val;
        }
        Ok(None)
    }

    fn render(&self, _frame: &mut Frame, _area: Rect) {
        // 由于使用了自定义的 render_with_state，这里留空
    }
}