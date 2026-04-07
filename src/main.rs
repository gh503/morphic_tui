// 程序的入口，负责初始化和生命周期
mod framework;
mod components;
mod apps;
mod app;
mod config;
mod database; 
mod models;   

use anyhow::Result;
use framework::*;
use app::{RootApp, ActiveApp}; 
use std::time::{Duration, Instant};
use tokio::sync::mpsc;
use std::panic;

#[tokio::main]
async fn main() -> Result<()> {
    // 1. 设置全局 Panic 钩子（确保崩溃时恢复终端状态）
    let default_hook = panic::take_hook();
    panic::set_hook(Box::new(move |panic_info| {
        let _ = ratatui::restore();
        let _ = crossterm::execute!(std::io::stdout(), crossterm::event::DisableMouseCapture);
        default_hook(panic_info);
    }));

    // 2. 初始化终端
    let mut terminal = ratatui::init();
    let _ = crossterm::execute!(std::io::stdout(), crossterm::event::EnableMouseCapture);

    // 4. 运行应用主循环
    let outcome = run_app(&mut terminal).await;

    // 5. 正常退出逻辑
    let _ = crossterm::execute!(std::io::stdout(), crossterm::event::DisableMouseCapture);
    ratatui::restore();

    std::process::exit(if outcome.is_ok() { 0 } else { 1 });
}

async fn run_app(terminal: &mut ratatui::DefaultTerminal) -> Result<()> {
    let (tx, mut rx) = mpsc::channel(100);
    let mut app = RootApp::new();

    // 1. 事件监听任务 (输入流)
    let event_tx = tx.clone();
    tokio::spawn(async move {
        loop {
            // 使用 100ms 轮询，既保证响应灵敏，又不至于让 CPU 忙等
            if crossterm::event::poll(Duration::from_millis(100)).unwrap() {
                if let Ok(ev) = crossterm::event::read() {
                    let event = match ev {
                        crossterm::event::Event::Key(key) => AppEvent::Key(key),
                        crossterm::event::Event::Mouse(mouse) => AppEvent::Mouse(mouse),
                        _ => continue,
                    };
                    if event_tx.send(event).await.is_err() { break; }
                }
            }
        }
    });

    let tick_rate = Duration::from_millis(200);
    let mut last_tick = Instant::now();
    let mut db_initialized = false;

    // 初始渲染：保证启动时屏幕不是空白
    terminal.draw(|f| app.render(f, f.area()))?;

    loop {
        // 2. 异步触发 Quality 数据库加载逻辑
        if app.active_tab == ActiveApp::Quality && !db_initialized {
            app.quality.is_loading = true;
            let refresh_tx = tx.clone();
            tokio::spawn(async move {
                // 模拟一个异步通知，避免在初始化时阻塞 UI
                let _ = refresh_tx.send(AppEvent::Action(CustomAction::RefreshData)).await;
            });
            db_initialized = true; 
        }

        // 3. 计算距离下一次 Tick 的剩余时间
        let timeout = tick_rate.saturating_sub(last_tick.elapsed());
        
        // 4. 事件驱动驱动核心：只在有变动时才唤醒
        tokio::select! {
            // A. 处理用户输入或自定义 Action
            Some(event) = rx.recv() => {
                match event {
                    // 全局退出
                    AppEvent::Key(k) if k.code == crossterm::event::KeyCode::Char('q') 
                        && app.quality.mode == apps::quality::QualityMode::Normal => {
                        app.save_config();
                        return Ok(());
                    },
                    
                    // 异步数据库同步逻辑
                    AppEvent::Action(CustomAction::RefreshData) => {
                        if let Err(e) = app.quality.ensure_db().await {
                            app.quality.error_msg = Some(format!("Init Error: {}", e));
                            app.quality.is_loading = false;
                        } else {
                            let tx_done = tx.clone();
                            tokio::spawn(async move {
                                tokio::time::sleep(Duration::from_millis(50)).await;
                                let _ = tx_done.send(AppEvent::Action(CustomAction::SyncDatabaseFinished)).await;
                            });
                        }
                    }

                    AppEvent::Action(CustomAction::SyncDatabaseFinished) => {
                        app.quality.is_loading = false;
                    }

                    // 常规业务事件分发
                    _ => {
                        if let Some(action) = app.handle_event(&event)? {
                            let _ = tx.send(AppEvent::Action(action)).await;
                        }
                    }
                }
                // ✅ 仅在处理完有效事件后重绘
                terminal.draw(|f| app.render(f, f.area()))?;
            }

            // B. 定时刷新 (Tick)
            _ = tokio::time::sleep(timeout) => {
                app.handle_event(&AppEvent::Tick)?;
                last_tick = Instant::now();
                // ✅ 仅在 Tick 到达时重绘
                terminal.draw(|f| app.render(f, f.area()))?;
            }
        }
    }
}