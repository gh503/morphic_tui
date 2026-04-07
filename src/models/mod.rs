// 1. 声明所有子模块，对应你的 01-07 SQL 结构
pub mod auth;
pub mod biz;
pub mod task;
pub mod notify;
pub mod quality;
pub mod acceptance;
pub mod assets;

// 2. 重新导出常用类型 (Re-exports)
// 这样在外部调用时可以用 models::User 而不是 models::auth::User
pub use auth::*;
pub use biz::*;
pub use task::*;
pub use notify::*;
pub use quality::*;
pub use acceptance::*;
pub use assets::*;

// 3. 定义通用的结果类型或错误类型（可选）
pub type DbResult<T> = Result<T, sqlx::Error>;

/// 辅助函数：快速检查数据库版本或初始化
pub async fn check_database_version(pool: &sqlx::SqlitePool) -> DbResult<String> {
    let row: (String,) = sqlx::query_as("SELECT sqlite_version()")
        .fetch_one(pool)
        .await?;
    Ok(row.0)
}