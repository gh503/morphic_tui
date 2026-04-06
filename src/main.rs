// 程序的入口，负责初始化和生命周期
mod framework;
mod components;
mod apps;
mod app;
mod config;

use anyhow::Result;
use framework::*;
use app::RootApp;
use std::time::{Duration, Instant};
use tokio::sync::mpsc;
use std::panic;

#[tokio::main]
async fn main() -> Result<()> {
    // 1. 设置全局 Panic 钩子，防止程序崩溃后终端字符乱码
    let default_hook = panic::take_hook();
    panic::set_hook(Box::new(move |panic_info| {
        let _ = ratatui::restore();
        let _ = crossterm::execute!(std::io::stdout(), crossterm::event::DisableMouseCapture);
        default_hook(panic_info);
    }));

    // 2. 初始化终端
    let mut terminal = ratatui::init();
    
    // 启用鼠标捕获（放在这里比放在 spawn 里更符合架构规范）
    crossterm::execute!(std::io::stdout(), crossterm::event::EnableMouseCapture)?;

    let outcome = run_app(&mut terminal).await;

    // 3. 正常退出逻辑
    let _ = crossterm::execute!(std::io::stdout(), crossterm::event::DisableMouseCapture);
    ratatui::restore();

    std::process::exit(if outcome.is_ok() { 0 } else { 1 });
}

async fn run_app(terminal: &mut ratatui::DefaultTerminal) -> Result<()> {
    let (tx, mut rx) = mpsc::channel(100);
    let mut app = RootApp::new();

    // 事件监听任务：统一处理键盘和鼠标
    let event_tx = tx.clone();
    tokio::spawn(async move {
        loop {
            // 使用 poll 避免 100% CPU 占用
            if crossterm::event::poll(Duration::from_millis(10)).unwrap() {
                match crossterm::event::read().unwrap() {
                    crossterm::event::Event::Key(key) => {
                        let _ = event_tx.send(AppEvent::Key(key)).await;
                    }
                    crossterm::event::Event::Mouse(mouse) => {
                        let _ = event_tx.send(AppEvent::Mouse(mouse)).await;
                    }
                    _ => {}
                }
            }
        }
    });

    // 采样与刷新控制
    let mut last_tick = Instant::now();
    let tick_rate = Duration::from_millis(200); // 5Hz 采样率，平衡流畅度与性能

    loop {
        // 1. 渲染当前状态
        terminal.draw(|f| app.render(f, f.area()))?;

        // 2. 计算等待超时的逻辑（处理动画平滑度）
        let timeout = if *app.is_animating.borrow() {
            Duration::from_millis(16) // 动画时约 60fps
        } else {
            tick_rate.saturating_sub(last_tick.elapsed())
        };

        // 3. 事件处理逻辑
        if let Ok(result) = tokio::time::timeout(timeout, rx.recv()).await {
            if let Some(event) = result {
                match event {
                    AppEvent::Key(k) if k.code == crossterm::event::KeyCode::Char('q') => {
                        app.save_config();
                        return Ok(());
                    },
                    _ => {
                        // 转发事件给 app，如果返回了 Action，则发回通道统一处理
                        if let Some(action) = app.handle_event(&event)? {
                            tx.send(AppEvent::Action(action)).await?;
                        }
                    }
                }
            }
        }

        // 4. 自动触发定时采样 (Tick)
        if last_tick.elapsed() >= tick_rate {
            app.handle_event(&AppEvent::Tick)?;
            last_tick = Instant::now();
        }
    }
}