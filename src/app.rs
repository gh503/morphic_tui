// src/app.rs
use crate::framework::*;
use crate::config::AppConfig;
use crate::apps::monitor::MonitorApp;
use crate::apps::settings::SettingsApp;
use crate::apps::info::InfoApp;
use crate::apps::quality::{QualityApp, QualityMode}; // ✅ 引入 QualityMode
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
    pub sidebar_width: u16,  
    pub target_sidebar_width: u16, 
    pub current_sidebar_width: RefCell<f32>, 
    pub is_animating: RefCell<bool>,
    pub last_size: RefCell<Rect>,
    pub is_dragging: bool,
    pub last_drag_time: Instant,
    pub config: AppConfig, 
}

#[derive(Debug, PartialEq, Eq, Clone)]
pub enum ActiveApp { Monitor, Settings, Info, Quality }

impl RootApp {
    pub fn new() -> Self {
        let mut config = confy::load::<AppConfig>("morphic_tui", None)
            .unwrap_or_else(|_| AppConfig::default());

        config.validate();

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
        let mut cfg = self.config.clone();
        cfg.sidebar_width = self.sidebar_width;
        cfg.show_sidebar = self.show_sidebar;
        cfg.max_points = self.monitor.max_points;
        let _ = confy::store("morphic_tui", None, cfg);
    }
    
    pub fn handle_event(&mut self, event: &AppEvent) -> anyhow::Result<Option<CustomAction>> {
        // --- 核心拦截：如果 Quality 正在录入，屏蔽 Root 层的全局按键 ---
        if let AppEvent::Key(_) = event {
            if self.active_tab == ActiveApp::Quality && self.quality.mode == QualityMode::Editing {
                return self.quality.handle_event(event);
            }
        }

        let mut pending_action = None;

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
                        self.save_config();
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
                        let tab_key = self.quality.get_current_tab_key(); // "projects", "tasks" 等

                        // 1. 改变配置状态 (None -> Asc -> Desc)
                        // 注意：这里一定要先 toggle，config 里的 sort 才会变
                        self.quality.toggle_sort(&mut self.config);

                        // 2. 拿到最新的列信息并强制排序
                        if let Some(cols) = self.config.table_columns.get(tab_key) {
                            let visible_cols: Vec<_> = cols.iter().filter(|c| c.visible).collect();
                            if let Some(target_col) = visible_cols.get(self.quality.column_index) {
                                
                                // --- 调试大法：如果你不确定这里是否执行，可以加一行 println ---
                                println!("Sorting Tab: {}, Col: {}, Order: {:?}", tab_key, target_col.name, target_col.sort);
                                
                                self.quality.apply_sort(tab_key, &target_col.name, target_col.sort.clone());
                            }
                        }

                        // 3. 退出 HeaderFocus 模式，回到列表浏览
                        self.quality.mode = QualityMode::Normal;

                        // 4. 持久化
                        let _ = self.config.save();
                    }
                    _ => {}
                }
            }
            _ => {}
        }

        let _ = self.sidebar.handle_event(event)?;

        // 分发给子 App
        let child_action = match self.active_tab {
            ActiveApp::Monitor => self.monitor.handle_event(event)?,
            ActiveApp::Info => self.info.handle_event(event)?,
            ActiveApp::Quality => self.quality.handle_event(event)?,
            ActiveApp::Settings => self.settings.handle_event(event)?,
        };

        // 处理最终 Action
        if let Some(action) = child_action.or(pending_action) {
            match &action {
                CustomAction::SetHistory(new_val) => {
                    self.config.max_points = *new_val;
                    if self.active_tab == ActiveApp::Settings {
                        self.monitor.handle_event(&AppEvent::Action(action.clone()))?;
                    }
                    self.save_config();
                }
                _ => {}
            }
            return Ok(Some(action));
        }

        Ok(None)
    }

    fn handle_mouse_logic(&mut self, mouse: MouseEvent, sidebar_area: Rect) -> Option<CustomAction> {
        let visual_width = sidebar_area.width;
        match mouse.kind {
            MouseEventKind::Down(MouseButton::Left) => {
                if mouse.column >= visual_width.saturating_sub(2) && mouse.column <= visual_width + 1 {
                    self.is_dragging = true;
                    self.is_animating.replace(false);
                    None
                } else if mouse.column < visual_width {
                    let relative_row = mouse.row.saturating_sub(sidebar_area.y);
                    match relative_row {
                        // 根据 Sidebar 布局映射点击区域切换 App
                        4..=5 => Some(CustomAction::NextApp), // 简化的逻辑
                        _ => None
                    }
                } else { None }
            }
            MouseEventKind::Up(MouseButton::Left) => {
                if self.is_dragging {
                    self.is_dragging = false;
                    self.sidebar_width = visual_width; 
                    self.target_sidebar_width = visual_width;
                    self.save_config();
                }
                None
            }
            MouseEventKind::Drag(MouseButton::Left) => {
                if self.is_dragging {
                    let new_w = mouse.column.clamp(10, 60);
                    self.sidebar_width = new_w;
                    self.current_sidebar_width.replace(new_w as f32);
                }
                None
            }
            _ => None,
        }
    }

    pub fn render(&self, f: &mut Frame, area: Rect) {
        self.last_size.replace(area);
        
        // 侧边栏动画逻辑
        let is_animating_val = *self.is_animating.borrow();
        if is_animating_val {
            let target = self.target_sidebar_width as f32;
            let current = *self.current_sidebar_width.borrow();
            let diff = target - current;
            if diff.abs() > 0.1 {
                self.current_sidebar_width.replace(current + diff * 0.25);
            } else {
                self.current_sidebar_width.replace(target);
                self.is_animating.replace(false);
            }
        }

        let current_w = *self.current_sidebar_width.borrow() as u16;
        let chunks = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Length(current_w), Constraint::Min(0)])
            .split(area);

        if current_w > 0 {
            let current_cpu = self.monitor.cpu_history.back().cloned().unwrap_or(0.0);
            self.sidebar.render_with_state(f, chunks[0], &self.active_tab, current_cpu);
            if self.is_dragging {
                f.render_widget(Block::default().borders(Borders::RIGHT).border_style(Style::default().fg(Color::Yellow)), chunks[0]);
            }
        }

        // 渲染主内容区
        match self.active_tab {
            ActiveApp::Monitor => self.monitor.render(f, chunks[1], &self.config),
            ActiveApp::Settings => self.settings.render(f, chunks[1], &self.config),
            ActiveApp::Info => self.info.render(f, chunks[1], &self.config),
            ActiveApp::Quality => self.quality.render(f, chunks[1], &self.config),
        }
    }
}