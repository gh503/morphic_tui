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
use std::collections::HashMap;

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

    pub config: AppConfig, // 保存全局配置对象，方便跨组件传递
}

#[derive(Debug, PartialEq, Eq, Clone)]
pub enum ActiveApp { Monitor, Settings, Info, Quality }

impl RootApp {
    pub fn new() -> Self {
        let config = confy::load::<AppConfig>("morphic_tui", None)
            .unwrap_or_else(|_| AppConfig::default());

        let initial_width = if config.show_sidebar { config.sidebar_width } else { 0 };
            
        Self {
            active_tab: ActiveApp::Monitor,
            show_sidebar: config.show_sidebar,
            monitor: MonitorApp::with_config(config.max_points),
            settings: SettingsApp { current_points: config.max_points },
            info: InfoApp::new(),
            quality: QualityApp::new(),
            sidebar: Sidebar::new(),
            sidebar_width: config.sidebar_width,
            
            is_dragging: false,
            last_drag_time: Instant::now(),
            target_sidebar_width: initial_width,
            current_sidebar_width: RefCell::new(initial_width as f32),
            is_animating: RefCell::new(false),
            last_size: RefCell::new(Rect::default()),
            config,
        }
    }

    pub fn save_config(&self) {
        // 同步最新的 UI 状态到 config 对象
        let mut cfg = self.config.clone();
        cfg.sidebar_width = self.sidebar_width;
        cfg.show_sidebar = self.show_sidebar;

        cfg.max_points = self.monitor.max_points;

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
                    },
                    CustomAction::SaveConfig => {
                        self.quality.toggle_sort(&mut self.config);
                        self.save_config();
                        return Ok(None);
                    }
                    _ => {}
                }
            }
            _ => {}
        }

        // --- 2. [核心优化] 转发给子组件：按需分发 ---
        
        // 侧边栏是全局组件，始终接收事件（用于处理内部动画或菜单点击）
        let _ = self.sidebar.handle_event(event)?;

        // 【关键修正】：使用 let child_action 接收 match 的返回值
        let child_action = match self.active_tab {
            ActiveApp::Monitor => self.monitor.handle_event(event)?,
            ActiveApp::Info => self.info.handle_event(event)?,
            ActiveApp::Quality => self.quality.handle_event(event)?,
            ActiveApp::Settings => self.settings.handle_event(event)?,
        };

        // --- 3. 统一 Action 后置处理 (解决一致性) ---
        // 无论是全局触发的还是子组件返回的 Action，统一在这里做最终状态同步
        if let Some(action) = child_action.or(pending_action) {
            match &action {
                CustomAction::SetHistory(new_val) => {
                    // 同步配置对象
                    self.config.max_points = *new_val;
                    // 只有当我们在 Settings 页面调整时，Monitor 是收不到事件的，所以强制转发一次
                    if self.active_tab == ActiveApp::Settings {
                        self.monitor.handle_event(&AppEvent::Action(action.clone()))?;
                    }
                    // 立即保存，防止丢失
                    self.save_config();
                }
                _ => {}
            }
            return Ok(Some(action));
        }

        Ok(None)
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
            ActiveApp::Monitor => self.monitor.render(f, chunks[1], &self.config),
            ActiveApp::Settings => self.settings.render(f, chunks[1], &self.config),
            ActiveApp::Info => self.info.render(f, chunks[1], &self.config),
            ActiveApp::Quality => self.quality.render(f, chunks[1], &self.config),
        }
    }
}