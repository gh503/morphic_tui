// 系统监控 (增加进程排序支持)
use crate::framework::*;
use std::collections::VecDeque;
use std::cell::RefCell;
use ratatui::{prelude::*, widgets::*, symbols::Marker};
use sysinfo::{System, ProcessRefreshKind};

#[derive(Default, PartialEq)]
pub enum SortBy {
    #[default]
    Cpu,
    Memory,
}

pub struct MonitorApp {
    pub cpu_history: VecDeque<f32>,
    pub mem_percent: f32,
    pub max_points: usize,
    pub sys: System,
    pub top_processes: Vec<(String, f32, u64)>, 
    pub cached_grid: RefCell<Vec<Vec<(f64, f64)>>>,
    pub last_max_points: RefCell<usize>,
    pub sort_by: SortBy, // 新增：排序维度
}

impl MonitorApp {
    pub fn new() -> Self {
        Self::with_config(50)
    }

    pub fn with_config(max_points: usize) -> Self {
        let mut sys = System::new_all();
        sys.refresh_cpu_all();
        
        Self { 
            cpu_history: VecDeque::from(vec![0.0; max_points]), 
            mem_percent: 0.0,
            max_points,
            sys,
            top_processes: Vec::new(),
            cached_grid: RefCell::new(Vec::new()),
            last_max_points: RefCell::new(0),
            sort_by: SortBy::Cpu,
        }
    }

    pub fn tick(&mut self) {
        self.sys.refresh_cpu_all(); 
        self.sys.refresh_memory();
        // 性能优化：仅刷新进程的 CPU 和内存，不刷新磁盘/网络
        self.sys.refresh_processes_specifics(
            sysinfo::ProcessesToUpdate::All,
            true,
            ProcessRefreshKind::nothing().with_cpu().with_memory()
        );

        let global_cpu = self.sys.global_cpu_usage();
        self.cpu_history.push_back(global_cpu);
        if self.cpu_history.len() > self.max_points {
            self.cpu_history.pop_front();
        }

        let used_mem = self.sys.used_memory();
        let total_mem = self.sys.total_memory();
        self.mem_percent = (used_mem as f32 / total_mem as f32) * 100.0;

        // --- 排序逻辑开始 ---
        let mut procs: Vec<_> = self.sys.processes().values().collect();
        
        match self.sort_by {
            SortBy::Cpu => {
                procs.sort_by(|a, b| b.cpu_usage().partial_cmp(&a.cpu_usage()).unwrap_or(std::cmp::Ordering::Equal));
            }
            SortBy::Memory => {
                procs.sort_by(|a, b| b.memory().cmp(&a.memory()));
            }
        }
        
        self.top_processes = procs.iter().take(30).map(|p| {
            (
                p.name().to_string_lossy().into_owned(),
                p.cpu_usage(),
                p.memory() / 1024 / 1024 
            )
        }).collect();
    }

    fn ensure_grid_cache(&self) {
        let mut last_points = self.last_max_points.borrow_mut();
        let mut grid = self.cached_grid.borrow_mut();

        if self.max_points == *last_points && !grid.is_empty() {
            return;
        }

        grid.clear();
        for y_val in [25, 50, 75, 100] {
            grid.push(vec![
                (0.0, y_val as f64), 
                (self.max_points as f64, y_val as f64)
            ]);
        }
        *last_points = self.max_points;
    }
}

impl Component for MonitorApp {
    fn handle_event(&mut self, event: &AppEvent) -> anyhow::Result<Option<CustomAction>> {
        match event {
            AppEvent::Tick => {
                self.tick();
            }
            AppEvent::Key(k) => {
                match k.code {
                    crossterm::event::KeyCode::Char('1') => self.sort_by = SortBy::Cpu,
                    crossterm::event::KeyCode::Char('2') => self.sort_by = SortBy::Memory,
                    _ => {}
                }
            }
            AppEvent::Action(CustomAction::SetHistory(new_val)) => {
                self.max_points = *new_val;
                while self.cpu_history.len() > self.max_points { self.cpu_history.pop_front(); }
                while self.cpu_history.len() < self.max_points { self.cpu_history.push_front(0.0); }
                // 强制重绘背景网格
                *self.last_max_points.borrow_mut() = 0;
            }
            _ => {}
        }
        Ok(None)
    }

    fn render(&self, frame: &mut Frame, area: Rect) {
        self.ensure_grid_cache();

        let chunks = Layout::vertical([
            Constraint::Length(3),  // 内存
            Constraint::Length(12), // CPU 图表
            Constraint::Min(0)      // 进程列表
        ]).split(area);

        // 1. 内存条渲染
        let gauge = Gauge::default()
            .block(Block::bordered().title(" 内存使用率 ").border_type(BorderType::Rounded))
            .gauge_style(Style::default().fg(if self.mem_percent > 80.0 { Color::Red } else { Color::Magenta }))
            .percent(self.mem_percent as u16);
        frame.render_widget(gauge, chunks[0]);

        // 2. CPU 图表渲染
        let cpu_points: Vec<(f64, f64)> = self.cpu_history.iter().enumerate()
            .map(|(i, &val)| (i as f64, val as f64)).collect();

        let grid = self.cached_grid.borrow();
        let mut datasets = Vec::new();
        for (i, _) in [25, 50, 75, 100].iter().enumerate() {
            datasets.push(Dataset::default()
                .marker(Marker::Dot)
                .style(Style::default().fg(Color::Rgb(50, 50, 50)))
                .data(&grid[i]));
        }

        datasets.push(Dataset::default()
            .name("CPU %")
            .marker(Marker::Braille)
            .graph_type(GraphType::Line)
            .style(Style::default().fg(Color::Yellow).bold())
            .data(&cpu_points));

        let chart = Chart::new(datasets)
            .block(Block::bordered().title(" CPU 实时负载 ").border_type(BorderType::Rounded))
            .x_axis(Axis::default().bounds([0.0, self.max_points as f64]))
            .y_axis(Axis::default().bounds([0.0, 100.0]).labels(vec![Line::from("0"), Line::from("50"), Line::from("100")]));
        frame.render_widget(chart, chunks[1]);

        // 3. 动态进程列表
        let displayable_rows = chunks[2].height.saturating_sub(3) as usize;
        
        // 动态表头：根据排序状态显示倒三角符号
        let cpu_header = if self.sort_by == SortBy::Cpu { "CPU (▼)" } else { "CPU" };
        let mem_header = if self.sort_by == SortBy::Memory { "内存 (▼)" } else { "内存" };

        let rows = self.top_processes.iter()
            .take(displayable_rows)
            .map(|(name, cpu, mem)| {
                Row::new(vec![
                    Cell::from(name.as_str()),
                    Cell::from(format!("{:.1}%", cpu)),
                    Cell::from(format!("{} MB", mem)),
                ])
            });

        let table_title = format!(
            " 进程监控 [ 1:按CPU排序 2:按内存排序 ] (Top {}) ", 
            self.top_processes.len().min(displayable_rows)
        );

        let table = Table::new(rows, [
            Constraint::Percentage(50),
            Constraint::Percentage(25),
            Constraint::Percentage(25),
        ])
        .header(
            Row::new(vec![
                Cell::from("进程名"),
                Cell::from(cpu_header).style(Style::default().fg(if self.sort_by == SortBy::Cpu { Color::Yellow } else { Color::Cyan })),
                Cell::from(mem_header).style(Style::default().fg(if self.sort_by == SortBy::Memory { Color::Yellow } else { Color::Cyan })),
            ])
            .style(Style::default().bold())
        )
        .block(Block::bordered()
            .title(table_title)
            .border_type(BorderType::Rounded)
            .border_style(Style::default().fg(if self.sort_by == SortBy::Cpu { Color::Yellow } else { Color::Magenta }))
        );
        
        frame.render_widget(table, chunks[2]);
    }
}