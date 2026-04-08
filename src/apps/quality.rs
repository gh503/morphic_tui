use crate::framework::*;
use crate::models::*;
use crate::database::Database;
use crate::config::{AppConfig, SortOrder};
use std::sync::Arc;
use sqlx::{SqlitePool, Row};
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
    Editing,
}

// --- 数据模型 ---
#[derive(sqlx::FromRow, Debug, Clone)]
pub struct TaskRecord {
    pub title: String,
    pub status: String,
    pub priority: i32,
}

#[derive(sqlx::FromRow, Debug, Clone)]
pub struct BugRecord {
    pub id: i32,
    pub title: String,
    pub severity: String,
    pub status: String,
}

pub struct QualityApp {
    pub mode: QualityMode,
    pub pool: Option<Arc<SqlitePool>>,
    pub is_loading: bool,
    pub active_tab: usize,
    
    // --- 交互状态 ---
    pub table_state: RefCell<TableState>,      
    pub column_index: usize,                   
    pub focus_on_detail: bool,                 
    
    // --- 缓存数据 ---
    pub projects: Vec<Project>,    
    pub tasks: Vec<TaskRecord>,       
    pub bugs: Vec<BugRecord>,        
    pub acceptance: Vec<String>,  
    pub assets: Vec<(String, String)>,         
    pub error_msg: Option<String>,
}

impl QualityApp {
    pub fn new() -> Self {
        let mut ts = TableState::default();
        ts.select(Some(0));

        Self {
            mode: QualityMode::Normal,
            pool: None,
            is_loading: false,
            active_tab: 0,
            table_state: RefCell::new(ts),
            column_index: 0,
            focus_on_detail: false,
            projects: Vec::new(),
            tasks: Vec::new(),
            bugs: Vec::new(),
            acceptance: Vec::new(),
            assets: Vec::new(),
            error_msg: None,
        }
    }

    fn get_current_tab_key(&self) -> &'static str {
        match self.active_tab {
            0 => "projects",
            1 => "tasks",
            2 => "bugs",      // Tab 3
            3 => "acceptance",
            4 => "assets",
            _ => "default",
        }
    }

    pub fn toggle_sort(&mut self, config: &mut AppConfig) {
        let key = self.get_current_tab_key();
        if let Some(cols) = config.table_columns.get_mut(key) {
            if self.column_index < cols.len() {
                if let Some(col) = cols.get_mut(self.column_index) {
                    col.sort = match col.sort {
                        SortOrder::None => SortOrder::Asc,
                        SortOrder::Asc => SortOrder::Desc,
                        SortOrder::Desc => SortOrder::None,
                    };
                    let order = col.sort.clone();
                    let col_name = col.name.clone();
                    self.apply_sort(key, &col_name, order);
                }
            }
        }
    }

    fn apply_sort(&mut self, key: &str, col_name: &str, order: SortOrder) {
        if order == SortOrder::None { return; }
        let asc = order == SortOrder::Asc;

        match key {
            "tasks" => self.tasks.sort_by(|a, b| {
                let res = match col_name.to_lowercase().as_str() {
                    "status" => a.status.cmp(&b.status),
                    "priority" => a.priority.cmp(&b.priority),
                    _ => a.title.cmp(&b.title),
                };
                if asc { res } else { res.reverse() }
            }),
            "bugs" => self.bugs.sort_by(|a, b| {
                let res = match col_name.to_lowercase().as_str() {
                    "id" => a.id.cmp(&b.id),
                    "级别" | "severity" => a.severity.cmp(&b.severity),
                    "状态" | "status" => a.status.cmp(&b.status),
                    _ => a.title.cmp(&b.title),
                };
                if asc { res } else { res.reverse() }
            }),
            _ => {}
        }
    }

    pub async fn refresh_data(&mut self) -> Result<()> {
        let pool = self.pool.as_ref().context("No DB Pool")?;

        // Projects
        self.projects = sqlx::query_as::<_, Project>("SELECT * FROM projects").fetch_all(&**pool).await.unwrap_or_default();
        
        // Tasks
        let task_rows = sqlx::query("SELECT title, status, priority FROM tasks").fetch_all(&**pool).await.unwrap_or_default();
        self.tasks = task_rows.iter().map(|r| TaskRecord {
            title: r.get("title"), status: r.get("status"), priority: r.get("priority")
        }).collect();

        // Bugs (Tab 3)
        let bug_rows = sqlx::query("SELECT id, title, severity, status FROM bugs").fetch_all(&**pool).await.unwrap_or_default();
        self.bugs = bug_rows.iter().map(|r| BugRecord {
            id: r.get("id"), title: r.get("title"), severity: r.get("severity"), status: r.get("status")
        }).collect();

        // Acceptance & Assets
        let acc_rows = sqlx::query("SELECT criteria FROM acceptance_criteria").fetch_all(&**pool).await.unwrap_or_default();
        self.acceptance = acc_rows.iter().map(|r| r.get::<String, _>("criteria")).collect();
        
        let asset_rows = sqlx::query("SELECT name, status FROM assets").fetch_all(&**pool).await.unwrap_or_default();
        self.assets = asset_rows.iter().map(|r| (r.get("name"), r.get("status"))).collect();

        Ok(())
    }

    pub async fn ensure_db(&mut self) -> Result<()> {
        if self.pool.is_some() { return Ok(()); }
        self.is_loading = true;
        match Database::init("sqlite:data/quality.sqlite").await {
            Ok(db) => {
                self.pool = Some(Arc::new(db.pool));
                self.refresh_data().await?;
                self.is_loading = false;
                Ok(())
            }
            Err(e) => {
                self.is_loading = false;
                self.error_msg = Some(e.to_string());
                Err(e)
            }
        }
    }

    fn render_master_table(&self, f: &mut Frame, area: Rect, config: &AppConfig) {
        let key = self.get_current_tab_key();
        
        // 关键修复：如果 config 里没有该 Tab 的配置，给一组默认展示列，防止界面空白
        let empty_vec = vec![];
        let cols = config.table_columns.get(key).unwrap_or(&empty_vec);
        
        if cols.is_empty() {
             f.render_widget(Paragraph::new(format!("请在 config.json 中配置 [{}] 的列信息", key)).dark_gray().alignment(Alignment::Center), area);
             return;
        }

        let header_cells = cols.iter().enumerate().map(|(i, c)| {
            let mut label = c.name.clone();
            match c.sort {
                SortOrder::Asc => label.push_str(" ▲"),
                SortOrder::Desc => label.push_str(" ▼"),
                _ => {}
            }
            let mut style = Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD);
            if self.mode == QualityMode::HeaderFocus && i == self.column_index {
                style = style.bg(Color::Cyan).fg(Color::Black);
            }
            Cell::from(label).style(style)
        });

        let rows: Vec<TuiRow> = match self.active_tab {
            0 => self.projects.iter().map(|p| TuiRow::new(vec![p.name.clone(), p.status.to_string()])).collect(),
            1 => self.tasks.iter().map(|t| TuiRow::new(vec![t.title.clone(), t.status.clone(), t.priority.to_string()])).collect(),
            2 => self.bugs.iter().map(|b| TuiRow::new(vec![b.id.to_string(), b.title.clone(), b.severity.clone(), b.status.clone()])).collect(),
            3 => self.acceptance.iter().map(|a| TuiRow::new(vec![a.clone()])).collect(),
            4 => self.assets.iter().map(|(n, s)| TuiRow::new(vec![n.clone(), s.clone()])).collect(),
            _ => vec![],
        };

        let widths: Vec<Constraint> = cols.iter().filter(|c| c.visible)
            .map(|c| Constraint::Percentage(c.width)).collect();

        let border_style = if !self.focus_on_detail { Style::default().fg(Color::Cyan) } else { Style::default().fg(Color::DarkGray) };

        let table = Table::new(rows, widths)
            .header(TuiRow::new(header_cells).height(1).bottom_margin(1))
            .block(Block::default().borders(Borders::ALL).title(format!(" {} ", key)).border_type(BorderType::Rounded).border_style(border_style))
            .highlight_style(Style::default().bg(Color::Rgb(50, 50, 50)))
            .highlight_symbol(">> ");

        f.render_stateful_widget(table, area, &mut self.table_state.borrow_mut());
    }

    fn render_detail_panel(&self, f: &mut Frame, area: Rect) {
        let style = if self.focus_on_detail { Style::default().fg(Color::Cyan) } else { Style::default().fg(Color::DarkGray) };
        let block = Block::default().borders(Borders::ALL).title(" 详细信息 ").border_type(BorderType::Rounded).border_style(style);

        let content = if let Some(idx) = self.table_state.borrow().selected() {
            match self.active_tab {
                0 => self.projects.get(idx).map(|p| vec![
                    Line::from(vec!["项目: ".into(), p.name.clone().yellow()]),
                    Line::from(vec!["状态: ".into(), p.status.to_string().green()]),
                ]),
                2 => self.bugs.get(idx).map(|b| vec![
                    Line::from(vec!["缺陷: ".into(), b.title.clone().red()]),
                    Line::from(vec!["严重度: ".into(), b.severity.clone().on_red()]),
                    Line::from(vec!["状态: ".into(), b.status.clone().cyan()]),
                ]),
                _ => Some(vec![Line::from("使用 j/k 浏览详情".italic().dark_gray())]),
            }
        } else { None };

        let widget = match content {
            Some(t) => Paragraph::new(t).block(block).wrap(Wrap { trim: true }),
            None => Paragraph::new("未选中数据").block(block).alignment(Alignment::Center),
        };
        f.render_widget(widget, area);
    }
}

// --- 实现 Component Trait (升级后版本) ---
impl Component for QualityApp {
    fn render(&self, f: &mut Frame, area: Rect, config: &AppConfig) {
        if self.is_loading {
            f.render_widget(Paragraph::new("正在拉取数据库数据...").yellow().alignment(Alignment::Center), area);
            return;
        }

        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Length(1), Constraint::Min(0)])
            .split(area);

        let titles = vec![" [1]项目 ", " [2]任务 ", " [3]质量 ", " [4]验收 ", " [5]资产 "];
        f.render_widget(
            Tabs::new(titles)
                .select(self.active_tab)
                .highlight_style(Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD))
                .divider("|"),
            chunks[0]
        );

        let work_layout = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Percentage(40), Constraint::Percentage(60)])
            .split(chunks[1]);

        self.render_master_table(f, work_layout[0], config);
        self.render_detail_panel(f, work_layout[1]);

        if let Some(ref err) = self.error_msg {
            let area = centered_rect(60, 10, area);
            f.render_widget(Clear, area);
            f.render_widget(Block::default().borders(Borders::ALL).title(" 异常提示 ").fg(Color::Red), area);
            f.render_widget(Paragraph::new(err.as_str()).alignment(Alignment::Center), area);
        }
    }

    fn handle_event(&mut self, event: &AppEvent) -> Result<Option<CustomAction>> {
        if let AppEvent::Key(key) = event {
            match key.code {
                KeyCode::Char('1') => { self.active_tab = 0; self.table_state.borrow_mut().select(Some(0)); return Ok(None); }
                KeyCode::Char('2') => { self.active_tab = 1; self.table_state.borrow_mut().select(Some(0)); return Ok(None); }
                KeyCode::Char('3') => { self.active_tab = 2; self.table_state.borrow_mut().select(Some(0)); return Ok(None); }
                KeyCode::Char('4') => { self.active_tab = 3; self.table_state.borrow_mut().select(Some(0)); return Ok(None); }
                KeyCode::Char('5') => { self.active_tab = 4; self.table_state.borrow_mut().select(Some(0)); return Ok(None); }
                _ => {}
            }

            match self.mode {
                QualityMode::Normal => {
                    match key.code {
                        KeyCode::Char('f') => self.mode = QualityMode::HeaderFocus,
                        KeyCode::Char('j') | KeyCode::Down => {
                            let len = match self.active_tab { 0 => self.projects.len(), 1 => self.tasks.len(), 2 => self.bugs.len(), 3 => self.acceptance.len(), 4 => self.assets.len(), _ => 0 };
                            if len > 0 {
                                let mut s = self.table_state.borrow_mut();
                                let i = match s.selected() { Some(i) => if i >= len - 1 { 0 } else { i + 1 }, None => 0 };
                                s.select(Some(i));
                            }
                        }
                        KeyCode::Char('k') | KeyCode::Up => {
                            let len = match self.active_tab { 0 => self.projects.len(), 1 => self.tasks.len(), 2 => self.bugs.len(), 3 => self.acceptance.len(), 4 => self.assets.len(), _ => 0 };
                            if len > 0 {
                                let mut s = self.table_state.borrow_mut();
                                let i = match s.selected() { Some(i) => if i == 0 { len - 1 } else { i - 1 }, None => 0 };
                                s.select(Some(i));
                            }
                        }
                        KeyCode::Left | KeyCode::Char('h') => self.focus_on_detail = false,
                        KeyCode::Right | KeyCode::Char('l') => self.focus_on_detail = true,
                        KeyCode::Char('r') => return Ok(Some(CustomAction::RefreshData)),
                        _ => {}
                    }
                }
                QualityMode::HeaderFocus => {
                    match key.code {
                        KeyCode::Char('f') | KeyCode::Esc => self.mode = QualityMode::Normal,
                        KeyCode::Left | KeyCode::Char('h') => { if self.column_index > 0 { self.column_index -= 1; } }
                        KeyCode::Right | KeyCode::Char('l') => { self.column_index += 1; } 
                        KeyCode::Enter => {
                            return Ok(Some(CustomAction::SaveConfig));
                        }
                        _ => {}
                    }
                }
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