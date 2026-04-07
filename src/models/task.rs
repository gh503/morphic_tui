use serde::{Deserialize, Serialize};
use sqlx::{FromRow, Type};
use chrono::{DateTime, Utc};

// --- 1. 任务状态与等级枚举 ---

/// 任务执行状态：严格对应 SQL CHECK 约束
#[derive(Debug, Clone, Serialize, Deserialize, Type, PartialEq, Eq)]
#[sqlx(rename_all = "PascalCase")]
pub enum TaskStatus {
    Todo,
    InProgress,
    Testing,
    Done,
    Blocked,
    Cancelled,
}

/// 任务优先级：1-3 映射
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[repr(i32)]
pub enum TaskPriority {
    Low = 1,
    Medium = 2,
    High = 3,
}

/// 风险等级：0-3 映射
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[repr(i32)]
pub enum RiskLevel {
    None = 0,
    Low = 1,
    Medium = 2,
    Critical = 3,
}

// --- 2. 核心任务模型 ---

/// 任务主表：记录任务的当前快照
#[derive(Debug, Clone, FromRow, Serialize, Deserialize)]
pub struct Task {
    pub id: String,
    pub milestone_id: String,
    pub assignee_id: Option<String>,
    pub title: String,
    pub content: Option<String>,
    pub status: TaskStatus,
    
    /// 优先级 (1:低, 2:中, 3:高)
    pub priority: i32,
    
    /// 风险等级 (0:正常, 3:极高)
    pub risk_level: i32,
    
    /// 进度 (0-100)
    pub progress: i32,
    
    pub deadline: Option<DateTime<Utc>>,
    
    // 审计字段
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub created_by: Option<String>,
    pub updated_by: Option<String>,
}

/// 任务流水日志：用于追踪进度变化及风险备注
#[derive(Debug, Clone, FromRow, Serialize, Deserialize)]
pub struct TaskJournal {
    pub id: String,
    pub task_id: String,
    pub updater_id: String,
    
    pub prev_status: Option<TaskStatus>,
    pub curr_status: TaskStatus,
    
    /// 进度变化量 (如 +10, -5)
    pub progress_delta: i32,
    
    /// 更新说明或风险备注
    pub content: Option<String>,
    
    pub created_at: DateTime<Utc>,
    pub created_by: Option<String>,
}

// --- 3. 业务聚合 DTO ---

/// 任务看板视图模型
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskCard {
    pub task: Task,
    pub assignee_name: Option<String>,
    pub milestone_title: Option<String>,
    pub is_overdue: bool, // 逻辑字段：当前时间是否超过 deadline
}