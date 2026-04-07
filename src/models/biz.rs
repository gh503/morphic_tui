use serde::{Deserialize, Serialize};
use sqlx::{FromRow, Type};
use chrono::{DateTime, Utc};
use rust_decimal::Decimal;
use std::fmt;

// --- 1. 约束枚举定义 ---

/// 产品型号生命周期状态
#[derive(Debug, Clone, Serialize, Deserialize, Type, PartialEq, Eq)]
#[sqlx(rename_all = "PascalCase")]
pub enum ModelStatus {
    Planning,
    Developing,
    Testing,
    Released,
    Maintained,
    EndOfLife,
}

/// 项目执行状态
#[derive(Debug, Clone, Serialize, Deserialize, Type, PartialEq, Eq)]
#[sqlx(rename_all = "PascalCase")]
pub enum ProjectStatus {
    Active,
    Archived,
    Completed,
    Suspended,
}

/// 里程碑达成状态
#[derive(Debug, Clone, Serialize, Deserialize, Type, PartialEq, Eq)]
#[sqlx(rename_all = "PascalCase")]
pub enum MilestoneStatus {
    Planned,
    InProgress,
    Achieved,
    Delayed,
    Cancelled,
}

// --- 2. 核心业务模型 ---

/// 产品线：业务顶层分类
#[derive(Debug, Clone, FromRow, Serialize, Deserialize)]
pub struct ProductLine {
    pub id: String,
    pub name: String,
    pub manager_id: Option<String>, // 关联 User.id
    pub description: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub created_by: Option<String>,
    pub updated_by: Option<String>,
}

/// 产品型号：具体的 SKU
#[derive(Debug, Clone, FromRow, Serialize, Deserialize)]
pub struct ProductModel {
    pub id: String,
    pub product_line_id: String,
    pub model_code: String,
    pub status: ModelStatus,        // 强类型约束
    pub description: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub created_by: Option<String>,
    pub updated_by: Option<String>,
}

/// 项目：研发任务的逻辑载体
#[derive(Debug, Clone, FromRow, Serialize, Deserialize)]
pub struct Project {
    pub id: String,
    pub model_id: String,
    pub name: String,
    pub status: ProjectStatus,      // 强类型约束
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub created_by: Option<String>,
    pub updated_by: Option<String>,
}

/// 里程碑：项目中的关键时间点
#[derive(Debug, Clone, FromRow, Serialize, Deserialize)]
pub struct Milestone {
    pub id: String,
    pub project_id: String,
    pub title: String,
    pub due_date: DateTime<Utc>,
    pub status: MilestoneStatus,    // 强类型约束
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub created_by: Option<String>,
    pub updated_by: Option<String>,
}

/// 多维经营指标：财务与生产的快照数据
#[derive(Debug, Clone, FromRow, Serialize, Deserialize)]
pub struct ProductMetrics {
    pub id: String,
    pub model_id: String,
    
    // 研发与生产 (REAL -> f64, INTEGER -> i64)
    pub rd_cost: Decimal,
    pub production_count: i64,
    pub unit_cost: Decimal,
    
    // 销售与财务
    pub sales_count: i64,
    pub revenue: Decimal,
    pub profit: Decimal,
    
    // 质量指标
    pub maintenance_count: i64,
    pub failure_rate: Decimal,
    
    pub snapshot_date: DateTime<Utc>,
    pub created_at: DateTime<Utc>,
    pub created_by: Option<String>,
}

impl fmt::Display for ProjectStatus {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        // 根据你的枚举定义进行匹配
        match self {
            ProjectStatus::Active => write!(f, "Active"),
            ProjectStatus::Archived => write!(f, "Archived"),
            ProjectStatus::Completed => write!(f, "Completed"),
            ProjectStatus::Suspended => write!(f, "Suspended"),
        }
    }
}