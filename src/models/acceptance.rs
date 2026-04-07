use serde::{Deserialize, Serialize};
use sqlx::{FromRow, Type};
use chrono::{DateTime, Utc};
use rust_decimal::Decimal;

// --- 1. 约束枚举定义 ---

/// 验收项类别
#[derive(Debug, Clone, Serialize, Deserialize, Type, PartialEq, Eq)]
#[sqlx(rename_all = "PascalCase")]
pub enum AcceptanceCategory {
    Hardware,
    Software,
    System,
    Compliance,
}

/// 验收发布类型
#[derive(Debug, Clone, Serialize, Deserialize, Type, PartialEq, Eq)]
pub enum ReleaseType {
    Alpha,
    Beta,
    RC, // Release Candidate
    GA, // General Availability
}

/// 准入结论
#[derive(Debug, Clone, Serialize, Deserialize, Type, PartialEq, Eq)]
#[sqlx(rename_all = "PascalCase")]
pub enum AcceptanceConclusion {
    Pending,
    Pass,
    ConditionalPass, // 带风险通过
    Fail,
}

/// 单项结果记录
#[derive(Debug, Clone, Serialize, Deserialize, Type, PartialEq, Eq)]
pub enum InspectionResult {
    Pass,
    Fail,
    #[sqlx(rename = "N/A")]
    NotApplicable,
}

// --- 2. 核心验收模型 ---

/// 验收标准项 (支持版本化与溯源)
#[derive(Debug, Clone, FromRow, Serialize, Deserialize)]
pub struct AcceptanceItem {
    pub id: String,
    pub base_item_id: String,     // 溯源ID，用于跨版本追踪
    pub category: AcceptanceCategory,
    pub version_tag: String,      // 标准版本号 (如: STD-2026-Q1)
    
    pub title: String,
    pub standard_desc: Option<String>,
    pub is_critical: bool,        // 一票否决标识
    pub is_active: bool,          // 当前标准是否生效
    
    // 审计字段
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub created_by: Option<String>,
    pub updated_by: Option<String>,
}

/// 验收任务批次 (Go/No-Go 决策载体)
#[derive(Debug, Clone, FromRow, Serialize, Deserialize)]
pub struct AcceptanceTask {
    pub id: String,
    pub model_id: String,
    pub version_name: String,     // 固件/软件版本号
    #[sqlx(rename = "type")]      // 处理 SQL 关键字
    pub release_type: ReleaseType,
    
    pub conclusion: AcceptanceConclusion,
    pub summary: Option<String>,
    
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub created_by: Option<String>,
    pub updated_by: Option<String>,
}

/// 验收执行明细
#[derive(Debug, Clone, FromRow, Serialize, Deserialize)]
pub struct AcceptanceRecord {
    pub id: String,
    pub task_id: String,
    pub item_id: String,          // 关联具体的标准项版本
    
    pub result: InspectionResult,
    pub measured_value: Option<String>, // 实测数据
    pub remark: Option<String>,
    pub evidence_url: Option<String>,   // 附件/报告链接
    
    pub updated_at: DateTime<Utc>,
    pub updated_by: Option<String>,
}

/// 版本演进评估统计
#[derive(Debug, Clone, FromRow, Serialize, Deserialize)]
pub struct AcceptanceEvaluation {
    pub id: String,
    pub task_id: String,
    pub pass_rate: Decimal,       // 通过率
    pub critical_fails: i64,      // 核心项失败数
    pub trend: Option<String>,    // 演进趋势描述
    pub created_at: DateTime<Utc>,
}