use serde::{Deserialize, Serialize};
use sqlx::{FromRow, Type};
use chrono::{DateTime, Utc};

// --- 1. 约束枚举定义 ---

/// 通知发送状态
#[derive(Debug, Clone, Serialize, Deserialize, Type, PartialEq, Eq)]
#[sqlx(rename_all = "PascalCase")]
pub enum NotifyStatus {
    Pending,        // 待发送
    Sent,           // 发送成功
    PartialFailed,  // 部分渠道失败
    Failed,         // 全部失败
}

/// 通知渠道类型
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "UPPERCASE")]
pub enum NotifyChannel {
    Email,
    System,   // 系统内部弹窗/消息
    Webhook,  // 外部机器人集成 (如飞书、钉钉)
}

// --- 2. 核心通知模型 ---

/// 通知队列：记录所有待发与已发的通知记录
#[derive(Debug, Clone, FromRow, Serialize, Deserialize)]
pub struct Notification {
    pub id: String,
    pub recipient_id: String,           // 接收人 ID
    pub task_id: Option<String>,        // 关联的任务 ID (可选)
    
    pub subject: String,                // 标题
    pub body: String,                   // 正文
    
    /// 渠道列表：使用 sqlx::types::Json 自动转换 SQL 中的 JSON 数组 ["EMAIL", "SYSTEM"]
    pub notify_channels: sqlx::types::Json<Vec<NotifyChannel>>,
    
    pub status: NotifyStatus,           // 发送状态
    
    pub retry_count: i64,               // 重试次数
    pub next_retry_at: Option<DateTime<Utc>>, // 下次重试时间 (由数据库触发器计算)
    
    // 审计字段 (通知通常不修改，仅记录创建)
    pub created_at: DateTime<Utc>,
    pub created_by: Option<String>,     // 触发通知的系统模块或用户
}

// --- 3. 业务聚合模型 (DTO) ---

/// 通知任务包：供发送引擎消费的数据结构
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NotifyTask {
    pub notification_id: String,
    pub recipient_email: Option<String>, // 从关联查询中获取
    pub recipient_webhook: Option<String>,
    pub content_subject: String,
    pub content_body: String,
}