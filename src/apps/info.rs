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
    // 静态硬件信息，启动时获取一次
    pub gpu_name: String,
    pub cpu_arch: String,
}

impl InfoApp {
    pub fn new() -> Self {
        let mut sys = System::new_all();
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
        }
    }

    /// 跨平台显卡探测：Windows 使用 WMIC，Linux 使用 nvidia-smi 或 lspci
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

    /// 字节单位换算
    fn format_bytes(bytes: u64) -> String {
        let b = bytes as f64;
        if b >= 1073741824.0 { format!("{:.2} GB", b / 1073741824.0) }
        else if b >= 1048576.0 { format!("{:.2} MB", b / 1048576.0) }
        else if b >= 1024.0 { format!("{:.2} KB", b / 1024.0) }
        else { format!("{} B", bytes) }
    }

    /// 运行时长格式化：Dd Hh Mm
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
            self.sys.refresh_memory();
            self.networks.refresh(true);
            
            if self.last_refresh.elapsed() >= Duration::from_secs(5) {
                self.disks.refresh(true);
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
        // 获取内存与 Swap 数据
        let total_mem = self.sys.total_memory();
        let used_mem = self.sys.used_memory();
        let total_swap = self.sys.total_swap();
        let used_swap = self.sys.used_swap();
        let hw_lines = vec![
            Line::from(vec![Span::raw(" CPU: "), Span::styled(cpu_brand, Style::default().fg(Color::Yellow).bold())]),
            Line::from(vec![Span::raw(" GPU: "), Span::styled(&self.gpu_name, Style::default().fg(Color::Magenta).bold())]),
            Line::from(vec![Span::raw(" 架构: "), Span::raw(&self.cpu_arch), Span::raw(format!(" ({} 核心)", self.sys.cpus().len()))]),
            // 内存显示优化：已用 / 总量
            Line::from(vec![
                Span::raw(" 内存: "), 
                Span::styled(
                    format!("{}/{}", Self::format_bytes(used_mem), Self::format_bytes(total_mem)), 
                    Style::default().fg(Color::Green)
                )
            ]),
            // Swap / Pagefile 显示：增加颜色预警
            Line::from(vec![
                Span::raw(" 虚拟内存: "), 
                Span::styled(
                    format!("{}/{}", Self::format_bytes(used_swap), Self::format_bytes(total_swap)), 
                    Style::default().fg(if used_swap > 0 { Color::Yellow } else { Color::DarkGray })
                ),
                Span::raw(if cfg!(target_os = "windows") { " (Pagefile)" } else { " (Swap)" }),
            ]),
        ];
        frame.render_widget(Paragraph::new(hw_lines).block(Block::bordered().title(" 核心硬件 ").border_style(Style::default().fg(Color::Yellow))), top_chunks[0]);

        // --- 2. 软件环境 (含格式化运行时长) ---
        let sw_lines = vec![
            Line::from(vec![Span::raw(" 操作系统: "), Span::styled(System::name().unwrap_or_default(), Style::default().fg(Color::Cyan).bold())]),
            Line::from(vec![Span::raw(" 内核版本: "), Span::raw(System::kernel_version().unwrap_or_default())]),
            Line::from(vec![Span::raw(" 主机名称: "), Span::raw(System::host_name().unwrap_or_default())]),
            Line::from(vec![Span::raw(" 运行时长: "), Span::styled(Self::format_uptime(System::uptime()), Style::default().fg(Color::Green))]),
            Line::from(vec![Span::raw(" 系统负载: "), Span::raw(format!("{:.2}", System::load_average().one))]),
        ];
        frame.render_widget(Paragraph::new(sw_lines).block(Block::bordered().title(" 软件环境 ").border_style(Style::default().fg(Color::Cyan))), top_chunks[1]);

        // --- 3. 存储资产 ---
        let mut disk_lines = Vec::new();
        for disk in &self.disks {
            let name = disk.name().to_string_lossy();
            let total = disk.total_space();
            let avail = disk.available_space();
            let usage = if total > 0 { ((total - avail) as f64 / total as f64) * 100.0 } else { 0.0 };

            disk_lines.push(Line::from(vec![
                Span::styled(format!(" 💿 {:<6}", name), Style::default().fg(Color::Green).bold()),
                Span::raw(format!(" {:>8} | 已用 {:.1}%", Self::format_bytes(total), usage)),
            ]));
        }
        frame.render_widget(Paragraph::new(disk_lines).block(Block::bordered().title(" 存储资产 ")), bottom_chunks[0]);

        // --- 4. 网络硬件标识 ---
        let mut net_lines = Vec::new();
        for (name, data) in &self.networks {
            let mac_str = match mac_address_by_name(name) {
                Ok(Some(ma)) => ma.to_string(),
                _ => "00:00:00:00:00:00".to_string(),
            };

            net_lines.push(Line::from(vec![
                Span::styled(format!(" 🌐 {:<6}", name), Style::default().fg(Color::Blue).bold()),
                Span::styled(format!(" MAC: {}", mac_str), Style::default().fg(Color::DarkGray)),
            ]));
            net_lines.push(Line::from(vec![
                Span::raw(format!("    ⬇ {} | ⬆ {}", Self::format_bytes(data.received()), Self::format_bytes(data.transmitted()))),
            ]));
        }
        frame.render_widget(Paragraph::new(net_lines).block(Block::bordered().title(" 网络硬件标识 ")), bottom_chunks[1]);
    }
}