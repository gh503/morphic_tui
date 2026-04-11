// 程序的入口，负责初始化和生命周期
mod framework;
mod components;
mod apps;
mod app;
mod config;
mod database; 
mod models;   
mod repositories; // ✅ 新增：引入数据仓库模块

use anyhow::Result;
use framework::*;
use app::{RootApp, ActiveApp}; 
use std::time::{Duration, Instant};
use tokio::sync::mpsc;
use std::panic;
use std::sync::Arc;

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

    // 3. 运行应用主循环
    let outcome = run_app(&mut terminal).await;

    // 4. 正常退出逻辑
    let _ = crossterm::execute!(std::io::stdout(), crossterm::event::DisableMouseCapture);
    ratatui::restore();

    std::process::exit(if outcome.is_ok() { 0 } else { 1 });
}

async fn run_app(terminal: &mut ratatui::DefaultTerminal) -> Result<()> {
    // 创建全局通信通道
    let (tx, mut rx) = mpsc::channel::<AppEvent>(100);
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

    // 初始渲染
    terminal.draw(|f| app.render(f, f.area()))?;

    loop {
        let mut should_render = false;

        // 2. 异步触发 Quality 数据库加载逻辑
        // 当切换到 Quality 标签页且未初始化时触发
        if app.active_tab == ActiveApp::Quality && !db_initialized {
            app.quality.is_loading = true;
            let refresh_tx = tx.clone();
            tokio::spawn(async move {
                // 触发确保数据库连接和 Repository 初始化的逻辑
                let _ = refresh_tx.send(AppEvent::Action(CustomAction::RefreshData)).await;
            });
            db_initialized = true; 
        }

        // 3. 计算下一次 Tick 的超时时间
        let timeout = tick_rate.saturating_sub(last_tick.elapsed());
        
        tokio::select! {
            // A. 处理用户输入或自定义 Action
            Some(event) = rx.recv() => {
                match event {
                    // 全局退出 (仅在 Normal 模式下允许，防止编辑时误触退出)
                    AppEvent::Key(k) if k.code == crossterm::event::KeyCode::Char('q') 
                        && app.quality.mode == apps::quality::QualityMode::Normal => {
                        app.save_config();
                        return Ok(());
                    },
                    
                    // 核心逻辑：刷新/初始化数据库
                    AppEvent::Action(CustomAction::RefreshData) => {
                        // ensure_db 内部现在会创建 Repository
                        if let Err(e) = app.quality.ensure_db().await {
                            app.quality.error_msg = Some(format!("Database Error: {}", e));
                            app.quality.is_loading = false;
                        } else {
                            // 刷新数据后，发送同步完成通知
                            let tx_done = tx.clone();
                            tokio::spawn(async move {
                                // 给予微小的延迟确保 UI 状态切换平滑
                                tokio::time::sleep(Duration::from_millis(20)).await;
                                let _ = tx_done.send(AppEvent::Action(CustomAction::SyncDatabaseFinished)).await;
                            });
                        }
                    }

                    // 数据库同步完成
                    AppEvent::Action(CustomAction::SyncDatabaseFinished) => {
                        app.quality.is_loading = false;
                        should_render = true;
                    }

                    // 保存配置 Action
                    AppEvent::Action(CustomAction::SaveConfig) => {
                        app.save_config();
                    }

                    // 常规业务事件分发
                    _ => {
                        if let Some(action) = app.handle_event(&event)? {
                            // 如果 handle_event 返回了 Action（例如 Enter 提交后触发刷新），将其送回队列
                            let _ = tx.send(AppEvent::Action(action)).await;
                        }
                    }
                }
                should_render = true;
            }

            // B. 定时刷新 (Tick)
            _ = tokio::time::sleep(timeout) => {
                app.handle_event(&AppEvent::Tick)?;
                last_tick = Instant::now();
                should_render = true;
            }
        }

        if should_render {
            terminal.draw(|f| app.render(f, f.area()))?;
        }
    }
}