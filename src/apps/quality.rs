use crate::framework::*;
use crate::models::*;
use crate::database::Database;
use std::sync::Arc;
use sqlx::{SqlitePool, Row};
use ratatui::{
    layout::{Constraint, Direction, Layout, Rect, Alignment},
    style::{Color, Modifier, Style},
    widgets::{Block, Borders, List, ListItem, Paragraph, Tabs, BorderType},
    Frame,
    prelude::Stylize,
};
use anyhow::Result;
use crossterm::event::{KeyCode, KeyEvent};

#[derive(Debug, PartialEq, Eq)] // 增加 PartialEq 方便模式判断
pub enum QualityMode {
    Normal,   // 浏览模式 (按 1-5 换 Tab)
    Editing,  // 编辑模式 (此时 q, b 等全局键应失效)
    Confirm,  // 确认模式 (二次确认弹窗)
}

pub struct QualityApp {
    pub mode: QualityMode,
    pub pool: Option<Arc<SqlitePool>>,
    pub is_loading: bool,
    pub active_tab: usize,
    // --- 动态数据缓存 ---
    pub projects: Vec<Project>,    
    pub tasks: Vec<String>,       
    pub bugs: Vec<String>,        
    pub acceptance: Vec<String>,  
    pub assets: Vec<String>,      
    pub error_msg: Option<String>,
}

impl QualityApp {
    pub fn new() -> Self {
        Self {
            mode: QualityMode::Normal,
            pool: None,
            is_loading: false,
            active_tab: 0,
            projects: Vec::new(),
            tasks: Vec::new(),
            bugs: Vec::new(),
            acceptance: Vec::new(),
            assets: Vec::new(),
            error_msg: None,
        }
    }

    pub async fn ensure_db(&mut self) -> Result<()> {
        if self.pool.is_some() { 
            return Ok(()); 
        }
        
        self.is_loading = true;
        let db_url = "sqlite:data/quality.sqlite"; 
        
        match Database::init(db_url).await {
            Ok(db) => {
                self.pool = Some(Arc::new(db.pool));
                self.error_msg = None; // 清除之前的错误
                // 注意：这里可以调用 refresh_data，但为了异步安全，建议让 main 循环控制
                self.refresh_data().await?;
            }
            Err(e) => {
                self.is_loading = false; // 出错要重置状态
                self.error_msg = Some(format!("DB Connect Failed: {}", e));
                return Err(anyhow::anyhow!("Database Connect error: {}", e));
            }
        }

        Ok(())
    }

    pub async fn refresh_data(&mut self) -> Result<()> {
        let pool = match &self.pool {
            Some(p) => p,
            None => return Ok(()),
        };

        // 1. 项目查询
        let project_query = sqlx::query_as::<_, Project>(
            r#"SELECT `id`, `model_id`, `name`, `status`, `created_at`, `updated_at`, `created_by`, `updated_by` FROM `projects`"#
        )
        .fetch_all(&**pool)
        .await;

        match project_query {
            Ok(data) => self.projects = data,
            Err(e) => self.error_msg = Some(format!("Projects Error: {}", e)),
        }

        // 2. 其他表数据拉取
        if let Ok(rows) = sqlx::query("SELECT `title` FROM `tasks` LIMIT 20").fetch_all(&**pool).await {
            self.tasks = rows.iter().map(|r| r.get::<String, _>("title")).collect();
        }

        if let Ok(rows) = sqlx::query("SELECT `title`, `severity` FROM `bugs` ORDER BY `severity` ASC").fetch_all(&**pool).await {
            self.bugs = rows.iter().map(|r| {
                let sev = r.try_get::<String, _>("severity").unwrap_or_else(|_| "N/A".into());
                let title = r.try_get::<String, _>("title").unwrap_or_else(|_| "Unknown".into());
                format!("[{}] {}", sev, title)
            }).collect();
        }

        if let Ok(rows) = sqlx::query("SELECT `criteria` FROM `acceptance_criteria`").fetch_all(&**pool).await {
            self.acceptance = rows.iter().map(|r| r.get::<String, _>("criteria")).collect();
        }

        if let Ok(rows) = sqlx::query("SELECT `name`, `status` FROM `assets`").fetch_all(&**pool).await {
            self.assets = rows.iter().map(|r| {
                format!("{} - {}", r.get::<String, _>("name"), r.get::<String, _>("status"))
            }).collect();
        }

        Ok(())
    }

    // --- 渲染辅助方法 ---
    fn render_biz_tab(&self, f: &mut Frame, area: Rect) {
        if self.projects.is_empty() {
            let empty_text = Paragraph::new("暂无项目数据，请检查数据库表 projects")
                .gray()
                .alignment(Alignment::Center);
            f.render_widget(empty_text, area);
            return;
        }
        let items: Vec<ListItem> = self.projects.iter()
            .map(|p| {
                let status_color = if p.status.to_string() == "Active" { Color::Green } else { Color::Gray };
                ListItem::new(format!(" 📦 {:<15} | {}", p.name, p.status)).style(Style::default().fg(status_color))
            }).collect();

        f.render_widget(List::new(items).block(Block::default().title(" 项目/型号列表 ").borders(Borders::ALL).border_type(BorderType::Rounded)).highlight_style(Style::default().add_modifier(Modifier::REVERSED)), area);
    }

    fn render_task_tab(&self, f: &mut Frame, area: Rect) {
        let items: Vec<ListItem> = self.tasks.iter().map(|t| ListItem::new(format!(" 🚀 {}", t)).cyan()).collect();
        f.render_widget(List::new(items).block(Block::default().title(" 研发流水线 ").borders(Borders::ALL)), area);
    }

    fn render_bug_tab(&self, f: &mut Frame, area: Rect) {
        let items: Vec<ListItem> = self.bugs.iter().map(|b| {
            let color = if b.contains("P0") { Color::Red } else { Color::Yellow };
            ListItem::new(format!(" 🐛 {}", b)).fg(color)
        }).collect();
        f.render_widget(List::new(items).block(Block::default().title(" 缺陷追踪 ").borders(Borders::ALL)), area);
    }

    fn render_accept_tab(&self, f: &mut Frame, area: Rect) {
        let items: Vec<ListItem> = self.acceptance.iter().map(|a| ListItem::new(format!(" ✅ {}", a)).white()).collect();
        f.render_widget(List::new(items).block(Block::default().title(" 验收标准 ").borders(Borders::ALL)), area);
    }

    fn render_asset_tab(&self, f: &mut Frame, area: Rect) {
        let items: Vec<ListItem> = self.assets.iter().map(|a| ListItem::new(format!(" 🤖 {}", a)).white()).collect();
        f.render_widget(List::new(items).block(Block::default().title(" 环境资产 ").borders(Borders::ALL)), area);
    }
}

impl Component for QualityApp {
    fn render(&self, f: &mut Frame, area: Rect) {
        if self.is_loading {
            let loading_text = Paragraph::new("正在拉取 AutoArk 质量数据...")
                .yellow().alignment(Alignment::Center).block(Block::default().borders(Borders::ALL));
            f.render_widget(loading_text, area);
            return;
        }

        let main_layout = Layout::default().constraints([Constraint::Length(1), Constraint::Min(0)]).split(area);
        let titles = vec![" [1]项目 ", " [2]任务 ", " [3]质量 ", " [4]验收 ", " [5]资产 "];
        
        f.render_widget(
            Tabs::new(titles).select(self.active_tab)
                .highlight_style(Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD)).divider("|"),
            main_layout[0]
        );

        let content_area = main_layout[1];
        if let Some(ref err) = self.error_msg {
            let chunks = Layout::default().constraints([Constraint::Length(3), Constraint::Min(0)]).split(content_area);
            f.render_widget(Paragraph::new(format!("⚠️ {}", err)).red().block(Block::default().borders(Borders::ALL)), chunks[0]);
            self.draw_tab_content(f, chunks[1]);
        } else {
            self.draw_tab_content(f, content_area);
        }

        // 如果处于编辑模式，可以在这里渲染一个弹窗 Overlay
        if self.mode == QualityMode::Editing {
            // TODO: 渲染弹出式编辑框
        }
    }

    fn handle_event(&mut self, event: &AppEvent) -> Result<Option<CustomAction>> {
        match event {
            // 1. 处理按键事件
            AppEvent::Key(key) => {
                // 如果处于编辑模式，按键逻辑应完全不同
                if self.mode == QualityMode::Editing {
                    if key.code == KeyCode::Esc {
                        self.mode = QualityMode::Normal;
                    }
                    // TODO: 处理编辑字符输入
                    return Ok(None);
                }

                // 正常模式下的按键处理
                match key.code {
                    KeyCode::Char('r') => return Ok(Some(CustomAction::RefreshData)),
                    KeyCode::Char('1') => self.active_tab = 0,
                    KeyCode::Char('2') => self.active_tab = 1,
                    KeyCode::Char('3') => self.active_tab = 2,
                    KeyCode::Char('4') => self.active_tab = 3,
                    KeyCode::Char('5') => self.active_tab = 4,
                    KeyCode::Tab => self.active_tab = (self.active_tab + 1) % 5,
                    KeyCode::Char('e') => self.mode = QualityMode::Editing, // 进入编辑模式示例
                    _ => {}
                }
                Ok(None)
            }
            
            // 2. 处理业务动作（意图驱动）
            AppEvent::Action(CustomAction::RefreshData) => {
                // 此处通常是在 main 循环中检测到此 Action 后，异步调用 refresh_data
                // 组件内部可以选择记录一个状态位，或者清理 error_msg
                self.error_msg = None;
                Ok(None)
            }
            
            _ => Ok(None)
        }
    }
}

impl QualityApp {
    fn draw_tab_content(&self, f: &mut Frame, area: Rect) {
        match self.active_tab {
            0 => self.render_biz_tab(f, area),
            1 => self.render_task_tab(f, area),
            2 => self.render_bug_tab(f, area),
            3 => self.render_accept_tab(f, area),
            4 => self.render_asset_tab(f, area),
            _ => {}
        }
    }
}