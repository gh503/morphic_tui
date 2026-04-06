// 负责组件的实例化和布局声明
use crate::framework::*;
use crate::config::AppConfig;
use crate::apps::monitor::MonitorApp;
use crate::apps::settings::SettingsApp;
use crate::components::Sidebar;
use ratatui::prelude::*;
use ratatui::widgets::{Block, Borders};
use crossterm::event::{MouseEvent, MouseEventKind, MouseButton};
use std::cell::RefCell;
use std::time::{Duration, Instant}; // 新增时间库

pub struct RootApp {
    pub active_tab: ActiveApp,
    pub show_sidebar: bool,
    pub monitor: MonitorApp,
    pub settings: SettingsApp,
    pub sidebar: Sidebar,
    pub sidebar_width: u16,  // 用户设定的目标常态宽度

    // 动画状态
    pub target_sidebar_width: u16, // 目标宽度（展开时的设定值，或 0）
    // 使用 RefCell 包裹动画中间状态
    pub current_sidebar_width: RefCell<f32>, 
    pub is_animating: RefCell<bool>,
    // 终端的大小
    pub last_size: RefCell<Rect>,
    
    // --- 性能优化：节流阀 ---
    pub is_dragging: bool,
    pub last_drag_time: Instant,
}

#[derive(PartialEq)]
pub enum ActiveApp { Monitor, Settings }

impl RootApp {
    pub fn new() -> Self {
        // 1. 尝试加载配置
        let config_result = confy::load::<AppConfig>("morphic_tui", None);

        // 2. 模式匹配：如果加载成功就用配置，失败（如第一次运行文件不存在）就用默认值
        let (monitor, settings_points, sidebar_w, show_sb) = match config_result {
            Ok(cfg) => {
                (
                    MonitorApp::with_config(cfg.max_points),
                    cfg.max_points,
                    cfg.sidebar_width,
                    cfg.show_sidebar,
                )
            }
            Err(_) => {
                // 第一次运行，配置不存在时的兜底逻辑
                (
                    MonitorApp::new(), // 调用刚才恢复的 new()
                    100,               // 默认点数
                    25,                // 默认侧边栏宽度
                    true,              // 默认显示侧边栏
                )
            }
        };

        let initial_width = if show_sb { sidebar_w } else { 0 };
        Self {
            active_tab: ActiveApp::Monitor,
            show_sidebar: show_sb,
            monitor,
            settings: SettingsApp { current_points: settings_points },
            sidebar: Sidebar::new(),
            sidebar_width: sidebar_w,
            
            // 动画与交互状态
            is_dragging: false,
            last_drag_time: Instant::now(),
            target_sidebar_width: initial_width,
            current_sidebar_width: RefCell::new(initial_width as f32),
            is_animating: RefCell::new(false),
            last_size: RefCell::new(Rect::default()),
        }
    }

    // 在程序退出前调用此方法保存
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

        match event {
            AppEvent::Mouse(mouse) => {
                // 1. 节流处理：针对拖拽事件，限制处理频率
                if let MouseEventKind::Drag(_) = mouse.kind {
                    if self.last_drag_time.elapsed() < Duration::from_millis(16) {
                        return Ok(None); 
                    }
                    self.last_drag_time = Instant::now();
                }

                // 2. 动态计算侧边栏 Rect (必须与 render 里的布局逻辑完全一致)
                let current_size = *self.last_size.borrow();
                let sidebar_area = if self.show_sidebar {
                    Layout::default()
                        .direction(Direction::Horizontal)
                        .constraints([
                            Constraint::Length(self.sidebar_width),
                            Constraint::Min(0),
                        ])
                        .split(current_size)[0] 
                } else {
                    Rect::default()
                };

                // 3. 传入 Rect 进行相对坐标判定
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
                        // 设置目标：如果显示则为设定的宽度，否则为 0
                        self.target_sidebar_width = if self.show_sidebar { self.sidebar_width } else { 0 };
                        self.is_animating.replace(true);
                    },
                    CustomAction::NextApp => {
                        self.active_tab = if self.active_tab == ActiveApp::Monitor {
                            ActiveApp::Settings
                        } else {
                            ActiveApp::Monitor
                        };
                    }
                    _ => {}
                }
            }
            _ => {}
        }

        // 4. 转发给子组件 (SDET 提醒: 注意处理子组件返回的 Action)
        let _ = self.monitor.handle_event(event)?;
        let _ = self.sidebar.handle_event(event)?;
        let a_settings = self.settings.handle_event(event)?;
        
        if a_settings.is_some() {
            return Ok(a_settings);
        }

        Ok(pending_action)
    }

    fn handle_mouse_logic(&mut self, mouse: MouseEvent, sidebar_area: Rect) -> Option<CustomAction> {
        if !self.show_sidebar { return None; }

        match mouse.kind {
            MouseEventKind::Down(MouseButton::Left) => {
                let edge = self.sidebar_width;
                
                // 判定拖拽边缘
                if mouse.column >= edge.saturating_sub(1) && mouse.column <= edge + 1 {
                    self.is_dragging = true;
                } 
                // 判定侧边栏内部点击
                else if mouse.column < edge {
                    // 计算相对于侧边栏顶部的相对行号
                    let relative_row = mouse.row.saturating_sub(sidebar_area.y);

                    // 基于 Sidebar 渲染布局的行号匹配
                    match relative_row {
                        4 => return Some(CustomAction::NextApp), // 或者直接修改 self.active_tab
                        5 => return Some(CustomAction::NextApp), // 同上
                        _ => {}
                    }
                }
            }
            MouseEventKind::Up(MouseButton::Left) => {
                self.is_dragging = false;
            }
            MouseEventKind::Drag(MouseButton::Left) => {
                if self.is_dragging {
                    self.sidebar_width = mouse.column.clamp(10, 60);
                }
            }
            _ => {}
        }
        None
    }

    pub fn render(&self, f: &mut Frame, area: Rect) {
        // 1. 每一帧都记录当前终端大小，用于事件坐标换算
        self.last_size.replace(area);

        // 2. 动画引擎：计算当前帧的侧边栏宽度
        // 使用内部可变性 (RefCell) 在 &self 方法中更新动画状态
        let is_animating_val = *self.is_animating.borrow();
        if is_animating_val {
            let target = self.target_sidebar_width as f32;
            let current = *self.current_sidebar_width.borrow();
            let diff = target - current;

            // 如果差距大于 0.1 像素，继续逼近
            if diff.abs() > 0.1 {
                let step = diff * 0.2; // 0.2 是平滑系数，数值越大动画越快
                self.current_sidebar_width.replace(current + step);
            } else {
                // 差距微小时直接锁定目标值，停止动画
                self.current_sidebar_width.replace(target);
                self.is_animating.replace(false);
            }
        }

        // 3. 获取当前帧的最终宽度（取整）
        let current_w = *self.current_sidebar_width.borrow() as u16;

        // 4. 定义布局
        // 侧边栏宽度由动画值 current_w 决定，右侧区域占据剩余所有空间
        let chunks = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([
                Constraint::Length(current_w),
                Constraint::Min(0),
            ])
            .split(area);

        // 5. 渲染侧边栏 (只要宽度大于 0 就要渲染)
        if current_w > 0 {
            let current_cpu = self.monitor.cpu_history.back().cloned().unwrap_or(0.0);
            self.sidebar.render_with_state(f, chunks[0], &self.active_tab, current_cpu);

            // 如果用户正在拖拽，在侧边栏右侧画一条黄色高亮线作为反馈
            if self.is_dragging {
                let drag_block = Block::default()
                    .borders(Borders::RIGHT)
                    .border_style(Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD));
                f.render_widget(drag_block, chunks[0]);
            }
        }

        // 6. 渲染主内容区域 (根据 active_tab 切换渲染组件)
        match self.active_tab {
            ActiveApp::Monitor => {
                self.monitor.render(f, chunks[1]);
            }
            ActiveApp::Settings => {
                // 假设 SettingsApp 也有一个 render 方法
                self.settings.render(f, chunks[1]);
            }
        }
    }
}