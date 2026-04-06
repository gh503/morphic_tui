// 框架设置 (调整历史点与计算说明)
use crate::framework::*;
use ratatui::{prelude::*, widgets::*};

pub struct SettingsApp {
    pub current_points: usize,
}

impl SettingsApp {
    pub fn new() -> Self {
        Self { current_points: 50 }
    }

    /// 计算当前配置下的物理覆盖时长 (假设 main.rs 的 tick 是 200ms)
    fn get_duration_info(&self) -> (f64, &str, Color) {
        let duration = (self.current_points as f64 * 0.2); // 200ms = 0.2s
        let (desc, color) = match self.current_points {
            n if n <= 40 => (" [ 瞬时模式 ] 捕捉极短时间的性能尖峰 (Spikes)", Color::Magenta),
            n if n <= 120 => (" [ 标准模式 ] 兼顾实时性与短期趋势 (Balanced)", Color::Cyan),
            _ => (" [ 趋势模式 ] 适合观察长时任务负载流向 (Trend)", Color::Green),
        };
        (duration, desc, color)
    }
}

impl Component for SettingsApp {
    fn handle_event(&mut self, event: &AppEvent) -> anyhow::Result<Option<CustomAction>> {
        if let AppEvent::Key(k) = event {
            match k.code {
                // 按向上键增加采样点，上限 200
                crossterm::event::KeyCode::Up => {
                    self.current_points = (self.current_points + 5).min(200);
                    return Ok(Some(CustomAction::SetHistory(self.current_points)));
                }
                // 按向下键减少采样点，下限 10
                crossterm::event::KeyCode::Down => {
                    self.current_points = (self.current_points.saturating_sub(5)).max(10);
                    return Ok(Some(CustomAction::SetHistory(self.current_points)));
                }
                _ => {}
            }
        }
        Ok(None)
    }

    fn render(&self, frame: &mut Frame, area: Rect) {
        // 将页面划分为上下两部分
        let chunks = Layout::vertical([
            Constraint::Length(10), // 配置区
            Constraint::Min(0),     // 动态说明区
        ]).split(area);

        // --- 1. 配置区渲染 ---
        let config_text = vec![
            Line::from(" [ 框架配置中心 ] ").alignment(Alignment::Center).style(Style::default().fg(Color::Magenta).bold()),
            Line::from(""),
            Line::from(vec![
                Span::raw("  当前 CPU 历史采样点: "),
                Span::styled(format!(" < {} > ", self.current_points), Style::default().fg(Color::Yellow).bold()),
                Span::raw(" 点"),
            ]),
            Line::from(""),
            Line::from(vec![
                Span::raw("  操作指南:"),
            ]),
            Line::from(vec![
                Span::styled("  ↑ / ↓ ", Style::default().fg(Color::Yellow)),
                Span::raw(" : 动态调整监控图表的横轴长度 (10-200)"),
            ]),
            Line::from(vec![
                Span::styled("  Tab   ", Style::default().fg(Color::Yellow)),
                Span::raw(" : 切换回监控应用查看实时效果"),
            ]),
        ];

        let config_p = Paragraph::new(config_text)
            .block(Block::bordered()
                .title(" 核心参数 ")
                .border_type(BorderType::Rounded)
                .border_style(Style::default().fg(Color::Magenta)));
        
        frame.render_widget(config_p, chunks[0]);

        // --- 2. 动态说明区渲染 ---
        let (duration, mode_desc, mode_color) = self.get_duration_info();
        
        let info_text = vec![
            Line::from(""),
            Line::from(vec![
                Span::styled(" [ 采样机制说明 ] ", Style::default().add_modifier(Modifier::UNDERLINED)),
            ]),
            Line::from(""),
            Line::from(vec![
                Span::raw(" • 覆盖时长: "),
                Span::styled(format!("{:.1} 秒", duration), Style::default().fg(Color::White).bold()),
                Span::raw(format!(" (计算公式: {} 点 × 0.2s)", self.current_points)),
            ]),
            Line::from(""),
            Line::from(vec![
                Span::raw(" • 当前模式: "),
                Span::styled(mode_desc, Style::default().fg(mode_color)),
            ]),
            Line::from(""),
            Line::from(vec![
                Span::styled(" [ SDET 性能视点 ] ", Style::default().add_modifier(Modifier::UNDERLINED)),
            ]),
            Line::from(" • 数据源: 基于 /proc/stat 的差值采样"),
            Line::from(" • 渲染压力: 随采样点线性增长 (Braille 绘制开销)"),
            Line::from(" • 内存状态: 环形队列存储，内存抖动极低"),
            Line::from(""),
            Line::from(" 注意: 调整将立即生效，且在应用间保持同步。").italic().style(Style::default().fg(Color::DarkGray)),
        ];

        let info_p = Paragraph::new(info_text)
            .block(Block::bordered()
                .title(" 实时计算与分析 ")
                .border_style(Style::default().fg(Color::DarkGray))
                .border_type(BorderType::Rounded))
            .wrap(Wrap { trim: true });

        frame.render_widget(info_p, chunks[1]);
    }
}