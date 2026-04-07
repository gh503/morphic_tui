use crate::framework::*;
use ratatui::{prelude::*, widgets::*};
use sysinfo::{Disks, Networks, System};
use mac_address::mac_address_by_name;
use std::time::{Duration, Instant};
use std::process::Command;

pub struct InfoApp {
    pub sys: System,
    pub disks: Disks,
    pub networks: Networks,
    pub last_refresh: Instant,
    // 静态硬件信息
    pub gpu_name: String,
    pub cpu_arch: String,
    // --- 优化点：数据缓存，避免在 render 中触发 IO ---
    pub uptime_cache: String,
    pub load_avg_cache: f64,
    pub host_name: String,
    pub kernel_version: String,
    pub os_name: String,
}

impl InfoApp {
    pub fn new() -> Self {
        // 精细化初始化：禁用进程扫描，只开启内存和基础 CPU 信息
        let mut sys = System::new_with_specifics(
            sysinfo::RefreshKind::nothing()
                .with_cpu(sysinfo::CpuRefreshKind::nothing().with_cpu_usage())
                .with_memory(sysinfo::MemoryRefreshKind::nothing().with_ram().with_swap()) 
        );
        sys.refresh_all();
        
        let disks = Disks::new_with_refreshed_list();
        let networks = Networks::new_with_refreshed_list();
        let cpu_arch = System::cpu_arch();

        Self {
            sys,
            disks,
            networks,
            last_refresh: Instant::now(),
            gpu_name: Self::detect_gpu(),
            cpu_arch,
            // 初始化缓存数据
            uptime_cache: Self::format_uptime(System::uptime()),
            load_avg_cache: System::load_average().one,
            host_name: System::host_name().unwrap_or_default(),
            kernel_version: System::kernel_version().unwrap_or_default(),
            os_name: System::name().unwrap_or_default(),
        }
    }

    fn detect_gpu() -> String {
        #[cfg(target_os = "windows")]
        {
            if let Ok(output) = Command::new("wmic")
                .args(["path", "win32_VideoController", "get", "name"])
                .output() 
            {
                let text = String::from_utf8_lossy(&output.stdout);
                let lines: Vec<&str> = text.lines()
                    .map(|s| s.trim())
                    .filter(|s| !s.is_empty() && *s != "Name")
                    .collect();
                if !lines.is_empty() { return lines.join(" | "); }
            }
        }

        #[cfg(target_os = "linux")]
        {
            if let Ok(output) = Command::new("nvidia-smi")
                .args(["--query-gpu=name", "--format=csv,noheader"])
                .output() 
            {
                let text = String::from_utf8_lossy(&output.stdout).trim().to_string();
                if !text.is_empty() { return text; }
            }
            if let Ok(output) = Command::new("sh")
                .arg("-c")
                .arg("lspci | grep -i vga | cut -d ':' -f3")
                .output() 
            {
                let text = String::from_utf8_lossy(&output.stdout).trim().to_string();
                if !text.is_empty() { return text; }
            }
        }
        "未检测到独立显卡".to_string()
    }

    fn format_bytes(bytes: u64) -> String {
        let b = bytes as f64;
        if b >= 1073741824.0 { format!("{:.2} GB", b / 1073741824.0) }
        else if b >= 1048576.0 { format!("{:.2} MB", b / 1048576.0) }
        else if b >= 1024.0 { format!("{:.2} KB", b / 1024.0) }
        else { format!("{} B", bytes) }
    }

    fn format_uptime(seconds: u64) -> String {
        let days = seconds / 86400;
        let hours = (seconds % 86400) / 3600;
        let minutes = (seconds % 3600) / 60;
        if days > 0 {
            format!("{}d {}h {}m", days, hours, minutes)
        } else {
            format!("{}h {}m", hours, minutes)
        }
    }
}

impl Component for InfoApp {
    fn handle_event(&mut self, event: &AppEvent) -> anyhow::Result<Option<CustomAction>> {
        if let AppEvent::Tick = event {
            // 200ms 刷新任务：仅限于纯内存操作或极轻量流量统计
            self.sys.refresh_memory();
            self.networks.refresh(false); // 关键：不重新扫描网卡，仅刷新流量
            
            // 5秒刷新任务：处理较重的 IO 操作
            if self.last_refresh.elapsed() >= Duration::from_secs(5) {
                self.disks.refresh(true);
                
                // 将原本在 render 里的 IO 调用移到这里，并存入缓存
                self.uptime_cache = Self::format_uptime(System::uptime());
                self.load_avg_cache = System::load_average().one;
                
                self.last_refresh = Instant::now();
            }
        }
        Ok(None)
    }

    fn render(&self, frame: &mut Frame, area: Rect) {
        let main_chunks = Layout::vertical([Constraint::Percentage(50), Constraint::Percentage(50)]).split(area);
        let top_chunks = Layout::horizontal([Constraint::Percentage(50), Constraint::Percentage(50)]).split(main_chunks[0]);
        let bottom_chunks = Layout::horizontal([Constraint::Percentage(50), Constraint::Percentage(50)]).split(main_chunks[1]);

        // --- 1. 核心硬件 ---
        let cpu_brand = self.sys.cpus().first().map(|c| c.brand()).unwrap_or("Unknown CPU");
        let hw_lines = vec![
            Line::from(vec![Span::raw(" CPU: "), Span::styled(cpu_brand, Style::default().fg(Color::Yellow).bold())]),
            Line::from(vec![Span::raw(" GPU: "), Span::styled(&self.gpu_name, Style::default().fg(Color::Magenta).bold())]),
            Line::from(vec![Span::raw(" 架构: "), Span::raw(&self.cpu_arch), Span::raw(format!(" ({} 核心)", self.sys.cpus().len()))]),
            Line::from(vec![
                Span::raw(" 内存: "), 
                Span::styled(format!("{}/{}", Self::format_bytes(self.sys.used_memory()), Self::format_bytes(self.sys.total_memory())), Style::default().fg(Color::Green))
            ]),
            Line::from(vec![
                Span::raw(" 虚拟内存: "), 
                Span::styled(format!("{}/{}", Self::format_bytes(self.sys.used_swap()), Self::format_bytes(self.sys.total_swap())), 
                Style::default().fg(if self.sys.used_swap() > 0 { Color::Yellow } else { Color::DarkGray })),
            ]),
        ];
        frame.render_widget(Paragraph::new(hw_lines).block(Block::bordered().title(" 核心硬件 ")), top_chunks[0]);

        // --- 2. 软件环境 (使用缓存数据) ---
        let sw_lines = vec![
            Line::from(vec![Span::raw(" 操作系统: "), Span::styled(&self.os_name, Style::default().fg(Color::Cyan).bold())]),
            Line::from(vec![Span::raw(" 内核版本: "), Span::raw(&self.kernel_version)]),
            Line::from(vec![Span::raw(" 主机名称: "), Span::raw(&self.host_name)]),
            Line::from(vec![Span::raw(" 运行时长: "), Span::styled(&self.uptime_cache, Style::default().fg(Color::Green))]),
            Line::from(vec![Span::raw(" 系统负载: "), Span::raw(format!("{:.2}", self.load_avg_cache))]),
        ];
        frame.render_widget(Paragraph::new(sw_lines).block(Block::bordered().title(" 软件环境 ")), top_chunks[1]);

        // --- 3. 存储资产 (render 仅负责遍历内存数据) ---
        let disk_lines: Vec<Line> = self.disks.iter().map(|disk| {
            let total = disk.total_space();
            let usage = if total > 0 { ((total - disk.available_space()) as f64 / total as f64) * 100.0 } else { 0.0 };
            Line::from(vec![
                Span::styled(format!(" 💿 {:<6}", disk.name().to_string_lossy()), Style::default().fg(Color::Green).bold()),
                Span::raw(format!(" {:>8} | 已用 {:.1}%", Self::format_bytes(total), usage)),
            ])
        }).collect();
        frame.render_widget(Paragraph::new(disk_lines).block(Block::bordered().title(" 存储资产 ")), bottom_chunks[0]);

        // --- 4. 网络硬件 ---
        let mut net_lines = Vec::new();
        for (name, data) in &self.networks {
            // 注意：mac_address_by_name 在 Linux 下也会触发文件读取
            // 严谨做法是也将其缓存，这里暂时保持不变，但建议后续优化
            net_lines.push(Line::from(vec![
                Span::styled(format!(" 🌐 {:<6}", name), Style::default().fg(Color::Blue).bold()),
                Span::raw(format!(" ⬇ {} | ⬆ {}", Self::format_bytes(data.received()), Self::format_bytes(data.transmitted()))),
            ]));
        }
        frame.render_widget(Paragraph::new(net_lines).block(Block::bordered().title(" 网络硬件标识 ")), bottom_chunks[1]);
    }
}