use sqlx::sqlite::{SqliteConnectOptions, SqlitePool, SqlitePoolOptions};
use std::str::FromStr;
use std::path::Path;
use tokio::fs;

/// 数据库连接包装器
pub struct Database {
    pub pool: SqlitePool,
}

impl Database {
    /// 初始化数据库连接并执行迁移
    pub async fn init(db_url: &str) -> anyhow::Result<Self> {
        // 1. 确保数据库文件所在的目录存在
        if let Some(path) = db_url.strip_prefix("sqlite:") {
            if let Some(parent) = Path::new(path).parent() {
                if !parent.exists() {
                    fs::create_dir_all(parent).await?;
                }
            }
        }

        // 2. 配置连接选项
        let connection_options = SqliteConnectOptions::from_str(db_url)?
            .create_if_missing(true)
            .foreign_keys(true); // 强制开启外键约束，这对你的 Quality 架构至关重要

        // 3. 创建连接池
        let pool = SqlitePoolOptions::new()
            .max_connections(5)
            .connect_with(connection_options)
            .await?;

        // 4. 执行自动迁移 (运行 migrations/ 文件夹下的所有 .sql)
        // 注意：sqlx 会按文件名顺序执行
        sqlx::migrate!("./migrations")
            .run(&pool)
            .await?;

        Ok(Self { pool })
    }
}