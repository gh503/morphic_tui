// 负责组件的实例化和布局声明
use crate::framework::*;
use crate::config::AppConfig;
use crate::apps::monitor::MonitorApp;
use crate::apps::settings::SettingsApp;
use crate::apps::info::InfoApp;
use crate::apps::quality::QualityApp;
use crate::components::Sidebar;
use ratatui::prelude::*;
use ratatui::widgets::{Block, Borders};
use crossterm::event::{MouseEvent, MouseEventKind, MouseButton};
use std::cell::RefCell;
use std::time::{Duration, Instant};

pub struct RootApp {
    pub active_tab: ActiveApp,
    pub show_sidebar: bool,
    pub monitor: MonitorApp,
    pub settings: SettingsApp,
    pub info: InfoApp,
    pub quality: QualityApp,
    pub sidebar: Sidebar,
    pub sidebar_width: u16,  // 用户设定的目标常态宽度

    // 动画状态
    pub target_sidebar_width: u16, 
    pub current_sidebar_width: RefCell<f32>, 
    pub is_animating: RefCell<bool>,
    pub last_size: RefCell<Rect>,
    
    // 交互节流与状态
    pub is_dragging: bool,
    pub last_drag_time: Instant,
}

#[derive(Debug, PartialEq, Eq, Clone)]
pub enum ActiveApp { Monitor, Settings, Info, Quality }

impl RootApp {
    pub fn new() -> Self {
        let config_result = confy::load::<AppConfig>("morphic_tui", None);

        let (monitor, settings_points, sidebar_w, show_sb) = match config_result {
            Ok(cfg) => (
                MonitorApp::with_config(cfg.max_points),
                cfg.max_points,
                cfg.sidebar_width,
                cfg.show_sidebar,
            ),
            Err(_) => (MonitorApp::new(), 100, 25, true),
        };

        let initial_width = if show_sb { sidebar_w } else { 0 };
        Self {
            active_tab: ActiveApp::Monitor,
            show_sidebar: show_sb,
            monitor,
            settings: SettingsApp { current_points: settings_points },
            info: InfoApp::new(),
            quality: QualityApp::new(),
            sidebar: Sidebar::new(),
            sidebar_width: sidebar_w,
            
            is_dragging: false,
            last_drag_time: Instant::now(),
            target_sidebar_width: initial_width,
            current_sidebar_width: RefCell::new(initial_width as f32),
            is_animating: RefCell::new(false),
            last_size: RefCell::new(Rect::default()),
        }
    }

    pub fn save_config(&self) {
        let cfg = AppConfig {
            sidebar_width: self.sidebar_width,
            max_points: self.monitor.max_points,
            show_sidebar: self.show_sidebar,
        };
        let _ = confy::store("morphic_tui", None, cfg);
    }
    
    pub fn handle_event(&mut self, event: &AppEvent) -> anyhow::Result<Option<CustomAction>> {
        let mut pending_action = None;

        // --- 1. 全局事件处理 (保持不变) ---
        match event {
            AppEvent::Mouse(mouse) => {
                if let MouseEventKind::Drag(_) = mouse.kind {
                    if self.last_drag_time.elapsed() < Duration::from_millis(16) {
                        return Ok(None); 
                    }
                    self.last_drag_time = Instant::now();
                }
                let visual_width = *self.current_sidebar_width.borrow() as u16;
                let current_size = *self.last_size.borrow();
                let sidebar_area = Rect::new(current_size.x, current_size.y, visual_width, current_size.height);
                pending_action = self.handle_mouse_logic(*mouse, sidebar_area);
            }
            AppEvent::Key(k) => {
                match k.code {
                    crossterm::event::KeyCode::Tab => pending_action = Some(CustomAction::NextApp),
                    crossterm::event::KeyCode::Char('b') => pending_action = Some(CustomAction::ToggleSidebar),
                    _ => {}
                }
            }
            AppEvent::Action(action) => {
                match action {
                    CustomAction::ToggleSidebar => {
                        self.show_sidebar = !self.show_sidebar;
                        self.target_sidebar_width = if self.show_sidebar { self.sidebar_width } else { 0 };
                        self.is_animating.replace(true);
                    },
                    CustomAction::NextApp => {
                        self.active_tab = match self.active_tab {
                            ActiveApp::Monitor => ActiveApp::Settings,
                            ActiveApp::Settings => ActiveApp::Info,
                            ActiveApp::Info => ActiveApp::Quality,
                            ActiveApp::Quality => ActiveApp::Monitor,
                        };
                    }
                    _ => {}
                }
            }
            _ => {}
        }

        // --- 2. [核心优化] 转发给子组件：按需分发 ---
        
        // 侧边栏是全局组件，始终接收事件（用于处理内部动画或菜单点击）
        let _ = self.sidebar.handle_event(event)?;

        // 仅将事件分发给当前活跃的 Tab
        // 这样当 active_tab 不是 Monitor 时，MonitorApp 就永远收不到 Tick，后台扫描彻底停止
        match self.active_tab {
            ActiveApp::Monitor => {
                if let Some(a) = self.monitor.handle_event(event)? { pending_action = Some(a); }
            }
            ActiveApp::Info => {
                if let Some(a) = self.info.handle_event(event)? { pending_action = Some(a); }
            }
            ActiveApp::Quality => {
                // QualityApp 如果有 Action 也需要接收
                let _ = self.quality.handle_event(event)?;
            }
            ActiveApp::Settings => {
                if let Some(a) = self.settings.handle_event(event)? { pending_action = Some(a); }
            }
        }

        Ok(pending_action)
    }

    fn handle_mouse_logic(&mut self, mouse: MouseEvent, sidebar_area: Rect) -> Option<CustomAction> {
        // 即使 show_sidebar 为 false，如果是动画中也应该允许判定（或者直接返回）
        let visual_width = sidebar_area.width;

        match mouse.kind {
            MouseEventKind::Down(MouseButton::Left) => {
                // 增加容错：边缘 2 个字符宽度内均判定为拖拽开始
                if mouse.column >= visual_width.saturating_sub(2) && mouse.column <= visual_width + 1 {
                    self.is_dragging = true;
                    // 开启拖拽时，关闭动画干扰，强制同步
                    self.is_animating.replace(false);
                } 
                else if mouse.column < visual_width {
                    // 侧边栏菜单点击 (基于相对行号)
                    let relative_row = mouse.row.saturating_sub(sidebar_area.y);
                    match relative_row {
                        // 这里的行号需根据 sidebar.rs 的实际渲染位置微调
                        4..=6 => return Some(CustomAction::NextApp),
                        _ => {}
                    }
                }
            }
            MouseEventKind::Up(MouseButton::Left) => {
                if self.is_dragging {
                    self.is_dragging = false;
                    // 释放时，同步目标宽度，防止动画回弹
                    self.sidebar_width = visual_width; 
                    self.target_sidebar_width = visual_width;
                }
            }
            MouseEventKind::Drag(MouseButton::Left) => {
                if self.is_dragging {
                    // 实时更新宽度并同步动画中间值
                    let new_w = mouse.column.clamp(10, 60);
                    self.sidebar_width = new_w;
                    self.current_sidebar_width.replace(new_w as f32);
                }
            }
            _ => {}
        }
        None
    }

    pub fn render(&self, f: &mut Frame, area: Rect) {
        self.last_size.replace(area);

        // 动画引擎
        let is_animating_val = *self.is_animating.borrow();
        if is_animating_val {
            let target = self.target_sidebar_width as f32;
            let current = *self.current_sidebar_width.borrow();
            let diff = target - current;

            if diff.abs() > 0.1 {
                let step = diff * 0.25; // 稍微调快一点响应
                self.current_sidebar_width.replace(current + step);
            } else {
                self.current_sidebar_width.replace(target);
                self.is_animating.replace(false);
            }
        }

        let current_w = *self.current_sidebar_width.borrow() as u16;

        let chunks = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([
                Constraint::Length(current_w),
                Constraint::Min(0),
            ])
            .split(area);

        if current_w > 0 {
            let current_cpu = self.monitor.cpu_history.back().cloned().unwrap_or(0.0);
            self.sidebar.render_with_state(f, chunks[0], &self.active_tab, current_cpu);

            // 拖拽视觉反馈：高亮右边界线
            if self.is_dragging {
                let drag_block = Block::default()
                    .borders(Borders::RIGHT)
                    .border_style(Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD));
                f.render_widget(drag_block, chunks[0]);
            }
        }

        match self.active_tab {
            ActiveApp::Monitor => self.monitor.render(f, chunks[1]),
            ActiveApp::Settings => self.settings.render(f, chunks[1]),
            ActiveApp::Info => self.info.render(f, chunks[1]),
            ActiveApp::Quality => self.quality.render(f, chunks[1]),
        }
    }
}