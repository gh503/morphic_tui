use crate::framework::*;
use crate::models::*;
use crate::database::Database;
use crate::config::{AppConfig, SortOrder};
use crate::repositories::quality_repo::QualityRepository; // 引入刚才分开的 Repo
use std::sync::Arc;
use ratatui::{
    layout::{Constraint, Direction, Layout, Rect, Alignment},
    style::{Color, Modifier, Style},
    widgets::{Block, Borders, Table, Row as TuiRow, Cell, Paragraph, Tabs, BorderType, TableState, Clear, Wrap},
    Frame,
    prelude::*,
};
use anyhow::{Result, Context};
use crossterm::event::KeyCode;
use std::cell::RefCell;

#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub enum QualityMode {
    Normal,
    HeaderFocus, 
    Filtering, 
    Editing,
}

// --- UI 专用的数据展示模型 ---
#[derive(Debug, Clone)]
pub struct TaskRecord {
    pub title: String,
    pub status: String,
    pub priority: i32,
}

#[derive(Debug, Clone)]
pub struct BugRecord {
    pub id: i32,
    pub title: String,
    pub severity: String,
    pub status: String,
}

pub struct QualityApp {
    pub mode: QualityMode,
    pub repo: Option<Arc<QualityRepository>>, // 核心：使用抽象后的 Repository
    pub is_loading: bool,
    pub active_tab: usize,
    
    // --- 交互状态 ---
    pub table_state: RefCell<TableState>,      
    pub column_index: usize,                   
    pub focus_on_detail: bool,                 
    pub filter_query: String,                  
    pub edit_buffer: String,       

    // --- 缓存数据（当前展示） ---
    pub projects: Vec<Project>,    
    pub tasks: Vec<TaskRecord>,       
    pub bugs: Vec<BugRecord>,        
    pub acceptance: Vec<String>,  
    pub assets: Vec<(String, String)>,         

    // --- 原始数据（用于过滤基准） ---
    raw_projects: Vec<Project>,
    raw_tasks: Vec<TaskRecord>,
    raw_bugs: Vec<BugRecord>,
    raw_acceptance: Vec<String>,
    raw_assets: Vec<(String, String)>,

    pub error_msg: Option<String>,
}

impl QualityApp {
    pub fn new() -> Self {
        let mut ts = TableState::default();
        ts.select(Some(0));

        Self {
            mode: QualityMode::Normal,
            repo: None,
            is_loading: false,
            active_tab: 0,
            table_state: RefCell::new(ts),
            column_index: 0,
            focus_on_detail: false,
            filter_query: String::new(),
            edit_buffer: String::new(),
            projects: Vec::new(), tasks: Vec::new(), bugs: Vec::new(),
            acceptance: Vec::new(), assets: Vec::new(),
            raw_projects: Vec::new(), raw_tasks: Vec::new(), raw_bugs: Vec::new(),
            raw_acceptance: Vec::new(), raw_assets: Vec::new(),
            error_msg: None,
        }
    }

    pub fn get_current_tab_key(&self) -> &'static str {
        match self.active_tab {
            0 => "projects", 1 => "tasks", 2 => "bugs", 3 => "acceptance", 4 => "assets", _ => "default",
        }
    }

    fn get_current_tab_max_col(&self) -> usize {
        match self.active_tab {
            0 => 1, // 项目: [0]名称, [1]状态
            1 => 2, // 任务: [0]标题, [1]状态, [2]优先级
            2 => 3, // 缺陷: [0]ID, [1]标题, [2]级别, [3]状态
            4 => 1, // 资产: [0]名称, [1]状态
            _ => 0,
        }
    }

    fn reset_view(&mut self) {
        self.table_state.borrow_mut().select(Some(0));
        self.column_index = 0;
        self.apply_filter();
    }

    pub fn toggle_sort(&self, config: &mut AppConfig) {
        let key = self.get_current_tab_key();
        if let Some(cols) = config.table_columns.get_mut(key) {
            // 必须按“可见列”的索引来找，因为 column_index 是 UI 层的索引
            let mut visible_indices: Vec<usize> = Vec::new();
            for (i, col) in cols.iter().enumerate() {
                if col.visible { visible_indices.push(i); }
            }
            
            if let Some(&actual_idx) = visible_indices.get(self.column_index) {
                let col = &mut cols[actual_idx];
                col.sort = match col.sort {
                    SortOrder::None => SortOrder::Asc,
                    SortOrder::Asc => SortOrder::Desc,
                    SortOrder::Desc => SortOrder::None,
                };
            }
        }
    }

    pub fn apply_sort(&mut self, _tab_key: &str, col_name: &str, order: SortOrder) {
        if order == SortOrder::None { return; } // 无排序则不处理
        let is_asc = order == SortOrder::Asc;
        let col_clean = col_name.trim();

        match self.active_tab {
            0 => match col_clean {
                "项目" => self.projects.sort_by(|a, b| if is_asc { a.name.cmp(&b.name) } else { b.name.cmp(&a.name) }),
                "状态" => self.projects.sort_by(|a, b| if is_asc { a.status.to_string().cmp(&b.status.to_string()) } else { b.status.to_string().cmp(&a.status.to_string()) }),
                _ => {}
            },
            1 => match col_clean {
                "任务标题" => self.tasks.sort_by(|a, b| if is_asc { a.title.cmp(&b.title) } else { b.title.cmp(&a.title) }),
                "状态" => self.tasks.sort_by(|a, b| if is_asc { a.status.cmp(&b.status) } else { b.status.cmp(&a.status) }),
                "优先级" => self.tasks.sort_by(|a, b| if is_asc { a.priority.cmp(&b.priority) } else { b.priority.cmp(&a.priority) }),
                _ => {}
            },
            2 => match col_clean {
                "ID" => self.bugs.sort_by(|a, b| if is_asc { a.id.cmp(&b.id) } else { b.id.cmp(&a.id) }),
                "标题" => self.bugs.sort_by(|a, b| if is_asc { a.title.cmp(&b.title) } else { b.title.cmp(&a.title) }),
                "级别" => self.bugs.sort_by(|a, b| if is_asc { a.severity.cmp(&b.severity) } else { b.severity.cmp(&a.severity) }),
                "状态" => self.bugs.sort_by(|a, b| if is_asc { a.status.cmp(&b.status) } else { b.status.cmp(&a.status) }),
                _ => {}
            },
            4 => match col_clean {
                "资产名称" | "Name" => self.assets.sort_by(|a, b| if is_asc { a.0.cmp(&b.0) } else { b.0.cmp(&a.0) }),
                "状态" | "Status" => self.assets.sort_by(|a, b| if is_asc { a.1.cmp(&b.1) } else { b.1.cmp(&a.1) }),
                _ => {}
            },
            _ => {}
        }
    }

    pub fn apply_filter(&mut self) {
        let query = self.filter_query.to_lowercase();
        if query.is_empty() {
            self.projects = self.raw_projects.clone();
            self.tasks = self.raw_tasks.clone();
            self.bugs = self.raw_bugs.clone();
            self.acceptance = self.raw_acceptance.clone();
            self.assets = self.raw_assets.clone();
        } else {
            match self.active_tab {
                0 => self.projects = self.raw_projects.iter().filter(|p| p.name.to_lowercase().contains(&query)).cloned().collect(),
                1 => self.tasks = self.raw_tasks.iter().filter(|t| t.title.to_lowercase().contains(&query)).cloned().collect(),
                2 => self.bugs = self.raw_bugs.iter().filter(|b| b.title.to_lowercase().contains(&query)).cloned().collect(),
                3 => self.acceptance = self.raw_acceptance.iter().filter(|a| a.to_lowercase().contains(&query)).cloned().collect(),
                4 => self.assets = self.raw_assets.iter().filter(|(n, _)| n.to_lowercase().contains(&query)).cloned().collect(),
                _ => {}
            }
        }
        self.validate_selection();
    }

    fn validate_selection(&self) {
        let len = match self.active_tab { 0 => self.projects.len(), 1 => self.tasks.len(), 2 => self.bugs.len(), 3 => self.acceptance.len(), 4 => self.assets.len(), _ => 0 };
        let mut s = self.table_state.borrow_mut();
        if len == 0 {
            s.select(None);
        } else if s.selected().unwrap_or(0) >= len {
            s.select(Some(0));
        }
    }

    pub async fn refresh_data(&mut self) -> Result<()> {
        let repo = self.repo.as_ref().context("Repository not ready")?;
        self.is_loading = true;

        // 显式标注类型帮助编译器
        self.raw_projects = repo.fetch_projects().await?;
        self.raw_tasks = repo.fetch_tasks().await?;
        self.raw_bugs = repo.fetch_bugs().await?;
        self.raw_acceptance = repo.fetch_acceptance().await?;
        self.raw_assets = repo.fetch_assets().await?;

        self.is_loading = false;
        self.apply_filter();
        Ok(())
    }

    pub async fn ensure_db(&mut self) -> Result<()> {
        if self.repo.is_some() {
            self.refresh_data().await?;
            return Ok(()); 
        }
        self.is_loading = true;
        match Database::init("sqlite:data/quality.sqlite").await {
            Ok(db) => {
                let pool = Arc::new(db.pool);
                self.repo = Some(Arc::new(QualityRepository::new(pool)));
                self.refresh_data().await?;
                Ok(())
            }
            Err(e) => {
                self.is_loading = false;
                self.error_msg = Some(e.to_string());
                Err(e)
            }
        }
    }

    // --- 渲染组件 ---

    fn render_editing_modal(&self, f: &mut Frame, area: Rect) {
        let popup_area = centered_rect(60, 25, area);
        f.render_widget(Clear, popup_area);

        let title = match self.active_tab {
            3 => " 新增验收标准 (Enter 提交 / Esc 取消) ",
            _ => " 快速录入 ",
        };

        let block = Block::default()
            .borders(Borders::ALL)
            .title(title)
            .border_type(BorderType::Double)
            .border_style(Style::default().fg(Color::Yellow));
        
        let inner = block.inner(popup_area);
        f.render_widget(block, popup_area);

        let p = Paragraph::new(format!("> {}_", self.edit_buffer))
            .wrap(Wrap { trim: true })
            .style(Style::default().fg(Color::White));
        f.render_widget(p, inner);
    }

    fn render_search_bar(&self, f: &mut Frame, area: Rect) {
        if self.mode == QualityMode::Filtering || !self.filter_query.is_empty() {
            let color = if self.mode == QualityMode::Filtering { Color::Magenta } else { Color::DarkGray };
            let block = Block::default()
                .borders(Borders::ALL)
                .border_type(BorderType::Rounded)
                .border_style(Style::default().fg(color))
                .title(" 实时搜索 ");
            f.render_widget(Paragraph::new(format!(" 🔍 {} ", self.filter_query)).block(block), area);
        }
    }

    fn render_master_table(&self, f: &mut Frame, area: Rect, config: &AppConfig) {
        let key = self.get_current_tab_key();
        let visible_cols: Vec<_> = config.table_columns.get(key)
            .cloned().unwrap_or_default()
            .into_iter().filter(|c| c.visible).collect();
        
        if visible_cols.is_empty() { return; }

        // 修复：Header 高亮逻辑
        let header = TuiRow::new(visible_cols.iter().enumerate().map(|(i, c)| {
            let mut label = format!(" {} ", c.name);
            match c.sort {
                SortOrder::Asc => label.push_str("▲"),
                SortOrder::Desc => label.push_str("▼"),
                SortOrder::None => label.push_str("  "), // 如果是 None，就留空
            }

            if self.mode == QualityMode::HeaderFocus && i == self.column_index {
                Cell::from(label).style(Style::default()
                    .bg(Color::Cyan)
                    .fg(Color::Black)
                    .add_modifier(Modifier::BOLD))
            } else {
                Cell::from(label).style(Style::default().fg(Color::Yellow))
            }

        })).height(1).bottom_margin(1);

        // 修复：确保 Tab 3/4 数据渲染匹配
        let rows: Vec<TuiRow> = match self.active_tab {
            0 => self.projects.iter().map(|p| TuiRow::new(vec![p.name.clone(), p.status.to_string()])).collect(),
            1 => self.tasks.iter().map(|t| TuiRow::new(vec![t.title.clone(), t.status.clone(), t.priority.to_string()])).collect(),
            2 => self.bugs.iter().map(|b| TuiRow::new(vec![b.id.to_string(), b.title.clone(), b.severity.clone(), b.status.clone()])).collect(),
            3 => self.acceptance.iter().map(|a| TuiRow::new(vec![a.clone()])).collect(),
            4 => self.assets.iter().map(|(n, s)| TuiRow::new(vec![n.clone(), s.clone()])).collect(),
            _ => vec![],
        };

        let t = Table::new(rows, visible_cols.iter().map(|c| Constraint::Percentage(c.width)).collect::<Vec<_>>())
            .header(header)
            .block(Block::default()
                .borders(Borders::ALL)
                .title(format!(" {} ", key.to_uppercase()))
                .border_type(BorderType::Rounded)
                .border_style(if !self.focus_on_detail { Color::Cyan } else { Color::DarkGray }))
            .highlight_symbol(">> ")
            .row_highlight_style(Style::default().bg(Color::Rgb(50, 50, 50)));

        f.render_stateful_widget(t, area, &mut self.table_state.borrow_mut());
    }

    fn render_detail_panel(&self, f: &mut Frame, area: Rect) {
        let block = Block::default()
            .borders(Borders::ALL)
            .title(" 详细信息 ")
            .border_type(BorderType::Rounded)
            .border_style(if self.focus_on_detail { Color::Cyan } else { Color::DarkGray });

        if let Some(idx) = self.table_state.borrow().selected() {
            let content = match self.active_tab {
                0 => self.projects.get(idx).map(|p| vec![Line::from(vec!["项目: ".into(), p.name.clone().yellow()]), Line::from(vec!["状态: ".into(), p.status.to_string().green()])]),
                1 => self.tasks.get(idx).map(|t| vec![Line::from(vec!["任务: ".into(), t.title.clone().yellow()]), Line::from(vec!["状态: ".into(), t.status.clone().cyan()])]),
                2 => self.bugs.get(idx).map(|b| vec![Line::from(vec!["缺陷: ".into(), b.title.clone().red()]), Line::from(vec!["状态: ".into(), b.status.clone().cyan()])]),
                3 => self.acceptance.get(idx).map(|a| vec![Line::from(vec!["验收标准: ".into(), a.clone().cyan()])]),
                _ => None,
            };
            if let Some(t) = content {
                f.render_widget(Paragraph::new(t).block(block).wrap(Wrap { trim: true }), area);
                return;
            }
        }
        f.render_widget(Paragraph::new("未选中数据").block(block).alignment(Alignment::Center).dark_gray(), area);
    }

    fn get_current_list_len(&self) -> usize {
        match self.active_tab {
            0 => self.projects.len(),
            1 => self.tasks.len(),
            2 => self.bugs.len(),
            3 => self.acceptance.len(),
            4 => self.assets.len(),
            _ => 0,
        }
    }
}

impl Component for QualityApp {
    fn render(&self, f: &mut Frame, area: Rect, config: &AppConfig) {
        if self.is_loading {
            f.render_widget(Paragraph::new("数据加载中...").yellow().alignment(Alignment::Center), area);
            return;
        }

        let main_chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(1), 
                Constraint::Min(0),    
                Constraint::Length(if self.mode == QualityMode::Filtering || !self.filter_query.is_empty() { 3 } else { 0 }) 
            ]).split(area);

        let titles = vec![" [1]项目 ", " [2]任务 ", " [3]质量 ", " [4]验收 ", " [5]资产 "];
        f.render_widget(Tabs::new(titles).select(self.active_tab).highlight_style(Style::default().fg(Color::Cyan).bold()).divider("|"), main_chunks[0]);

        let work_layout = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Percentage(45), Constraint::Percentage(55)])
            .split(main_chunks[1]);

        self.render_master_table(f, work_layout[0], config);
        self.render_detail_panel(f, work_layout[1]);
        self.render_search_bar(f, main_chunks[2]);

        // 顶层渲染弹窗
        if self.mode == QualityMode::Editing {
            self.render_editing_modal(f, area);
        }
    }

    fn handle_event(&mut self, event: &AppEvent) -> Result<Option<CustomAction>> {
        if let AppEvent::Key(key) = event {
            match self.mode {
                QualityMode::Normal => match key.code {
                    // Tab 切换
                    KeyCode::Char('1') => { self.active_tab = 0; self.reset_view(); }
                    KeyCode::Char('2') => { self.active_tab = 1; self.reset_view(); }
                    KeyCode::Char('3') => { self.active_tab = 2; self.reset_view(); }
                    KeyCode::Char('4') => { self.active_tab = 3; self.reset_view(); }
                    KeyCode::Char('5') => { self.active_tab = 4; self.reset_view(); }

                    // 功能按键
                    KeyCode::Char('/') => self.mode = QualityMode::Filtering,
                    KeyCode::Char('s') => { // 排序触发
                        // 进入排序模式，并将焦点初始化到第一列
                        self.mode = QualityMode::HeaderFocus;
                        self.column_index = 0; 
                    }
                    KeyCode::Char('r') => return Ok(Some(CustomAction::RefreshData)),
                    KeyCode::Char('i') | KeyCode::Char('a') => {
                        self.mode = QualityMode::Editing;
                        self.edit_buffer.clear();
                    }

                    // 上下移动
                    KeyCode::Char('j') | KeyCode::Down => {
                        let len = self.get_current_list_len();
                        if len > 0 {
                            let mut s = self.table_state.borrow_mut();
                            let i = match s.selected() { Some(i) => if i >= len - 1 { 0 } else { i + 1 }, None => 0 };
                            s.select(Some(i));
                        }
                    }
                    KeyCode::Char('k') | KeyCode::Up => {
                        let len = self.get_current_list_len();
                        if len > 0 {
                            let mut s = self.table_state.borrow_mut();
                            let i = match s.selected() { Some(i) => if i == 0 { len - 1 } else { i - 1 }, None => 0 };
                            s.select(Some(i));
                        }
                    }

                    // 左右移动：修复排序焦点逻辑
                    KeyCode::Left | KeyCode::Char('h') => {
                        self.focus_on_detail = false;
                    }
                    KeyCode::Right | KeyCode::Char('l') => {
                        self.focus_on_detail = true;
                    }
                    _ => {}
                },
                QualityMode::Filtering => match key.code {
                    KeyCode::Esc | KeyCode::Enter => self.mode = QualityMode::Normal,
                    KeyCode::Backspace => { self.filter_query.pop(); self.apply_filter(); }
                    KeyCode::Char(c) => { self.filter_query.push(c); self.apply_filter(); }
                    _ => {}
                },
                QualityMode::Editing => match key.code {
                    KeyCode::Esc => self.mode = QualityMode::Normal,
                    KeyCode::Enter => {
                        if !self.edit_buffer.is_empty() {
                            let content = self.edit_buffer.clone();
                            let repo = self.repo.clone();
                            let tab = self.active_tab;
                            tokio::spawn(async move {
                                if let (Some(r), 3) = (repo, tab) {
                                    let _ = r.add_acceptance(&content).await;
                                }
                            });
                        }
                        self.mode = QualityMode::Normal;
                        return Ok(Some(CustomAction::RefreshData));
                    }
                    KeyCode::Backspace => { self.edit_buffer.pop(); }
                    KeyCode::Char(c) => { self.edit_buffer.push(c); }
                    _ => {}
                },
                QualityMode::HeaderFocus => match key.code {
                    KeyCode::Left | KeyCode::Char('h') => {
                        if self.column_index > 0 { self.column_index -= 1; }
                    }
                    KeyCode::Right | KeyCode::Char('l') => {
                        let max_col = self.get_current_tab_max_col(); // 辅助方法
                        if self.column_index < max_col { self.column_index += 1; }
                    }
                    KeyCode::Enter => {
                        // 执行排序保存逻辑
                        return Ok(Some(CustomAction::SaveConfig));
                    }
                    KeyCode::Esc | KeyCode::Char('q') | KeyCode::Char('s') => {
                        // 再次按 s 或 Esc 退出排序模式
                        self.mode = QualityMode::Normal;
                    }
                    _ => {}
                },
                _ => {}
            }
        }
        Ok(None)
    }
}

fn centered_rect(percent_x: u16, percent_y: u16, r: Rect) -> Rect {
    let popup_layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Percentage((100 - percent_y) / 2), Constraint::Percentage(percent_y), Constraint::Percentage((100 - percent_y) / 2)])
        .split(r);
    Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage((100 - percent_x) / 2), Constraint::Percentage(percent_x), Constraint::Percentage((100 - percent_x) / 2)])
        .split(popup_layout[1])[1]
}