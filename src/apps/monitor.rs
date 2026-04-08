// 系统监控 (优化版：精细化刷新与性能隔离)
use crate::framework::*;
use crate::config::AppConfig;
use std::collections::VecDeque;
use std::cell::RefCell;
use ratatui::{prelude::*, widgets::*, symbols::Marker};
use sysinfo::{System, ProcessRefreshKind, CpuRefreshKind, MemoryRefreshKind, RefreshKind, ProcessesToUpdate};

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
    pub sort_by: SortBy,
}

impl MonitorApp {
    pub fn new() -> Self {
        Self::with_config(50)
    }

    pub fn with_config(max_points: usize) -> Self {
        // --- 优化 1：精细化初始化，彻底禁用不相关的内核扫描 (磁盘、网络、用户等) ---
        let mut sys = System::new_with_specifics(
            RefreshKind::nothing()
                .with_cpu(CpuRefreshKind::nothing().with_cpu_usage())
                .with_memory(MemoryRefreshKind::nothing().with_ram())
                .with_processes(ProcessRefreshKind::nothing().with_cpu().with_memory())
        );

        // 初始刷新
        sys.refresh_cpu_usage();

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
        // --- 优化 2：按需刷新，绝不调用 refresh_all() ---
        self.sys.refresh_cpu_usage(); 
        self.sys.refresh_memory();
        
        // 仅刷新进程的 CPU 和内存，这是 proc_tid_stat 的来源，必须受控
        self.sys.refresh_processes_specifics(
            ProcessesToUpdate::All,
            true, // 清理已退出的进程
            ProcessRefreshKind::nothing().with_cpu().with_memory()
        );

        // 更新全局 CPU 历史
        let global_cpu = self.sys.global_cpu_usage();
        self.cpu_history.push_back(global_cpu);
        if self.cpu_history.len() > self.max_points {
            self.cpu_history.pop_front();
        }

        // 更新内存百分比
        let total_mem = self.sys.total_memory();
        if total_mem > 0 {
            self.mem_percent = (self.sys.used_memory() as f32 / total_mem as f32) * 100.0;
        }

        // --- 优化 3：零拷贝排序与高效截断 ---
        // 提取引用进行排序，避免在排序阶段克隆字符串
        let mut procs: Vec<_> = self.sys.processes().values().collect();
        
        match self.sort_by {
            SortBy::Cpu => {
                // 使用 sort_unstable_by 提升 20% 以上的排序性能
                procs.sort_unstable_by(|a, b| {
                    b.cpu_usage().partial_cmp(&a.cpu_usage()).unwrap_or(std::cmp::Ordering::Equal)
                });
            }
            SortBy::Memory => {
                procs.sort_unstable_by(|a, b| b.memory().cmp(&a.memory()));
            }
        }
        
        // 仅对最终展示的前 N 个进程进行数据转换
        self.top_processes = procs.iter().take(30).map(|p| {
            (
                p.name().to_string_lossy().to_string(),
                p.cpu_usage(),
                p.memory() / 1024 / 1024 // 转换为 MB
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
            // 关键：只有当前 Tab 活跃时，App.rs 才会分发 Tick 到这里
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
                // 调整队列长度
                while self.cpu_history.len() > self.max_points { self.cpu_history.pop_front(); }
                while self.cpu_history.len() < self.max_points { self.cpu_history.push_front(0.0); }
                *self.last_max_points.borrow_mut() = 0; // 触发网格重绘
            }
            _ => {}
        }
        Ok(None)
    }

    fn render(&self, frame: &mut Frame, area: Rect, config: &AppConfig) {
        self.ensure_grid_cache();

        let chunks = Layout::vertical([
            Constraint::Length(3),  // 内存条
            Constraint::Length(12), // CPU 图表
            Constraint::Min(0)      // 进程列表
        ]).split(area);

        // 1. 内存条
        let gauge = Gauge::default()
            .block(Block::bordered().title(" 内存使用率 ").border_type(BorderType::Rounded))
            .gauge_style(Style::default().fg(if self.mem_percent > 80.0 { Color::Red } else { Color::Magenta }))
            .percent(self.mem_percent as u16);
        frame.render_widget(gauge, chunks[0]);

        // 2. CPU 图表 (使用缓存的背景网格)
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
            .x_axis(Axis::default().bounds([0.0, config.max_points as f64]))
            .y_axis(Axis::default().bounds([0.0, 100.0]).labels(vec![Line::from("0"), Line::from("50"), Line::from("100")]));
        frame.render_widget(chart, chunks[1]);

        // 3. 进程列表 (Table 渲染)
        let displayable_rows = chunks[2].height.saturating_sub(3) as usize;
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

        let table = Table::new(rows, [
            Constraint::Percentage(50),
            Constraint::Percentage(25),
            Constraint::Percentage(25),
        ])
        .header(
            Row::new(vec![Cell::from("进程名"), Cell::from(cpu_header), Cell::from(mem_header)])
                .style(Style::default().bold().fg(Color::Cyan))
        )
        .block(Block::bordered()
            .title(format!(" 进程监控 (Top {}) ", self.top_processes.len().min(displayable_rows)))
            .border_type(BorderType::Rounded));
        
        frame.render_widget(table, chunks[2]);
    }
}