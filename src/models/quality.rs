use serde::{Deserialize, Serialize};
use sqlx::{FromRow, Type};
use chrono::{DateTime, Utc};
use rust_decimal::Decimal; // 承接精度要求

// --- 1. 约束枚举定义 ---

/// 质量阶段：定义缺陷发生的生命周期位置
#[derive(Debug, Clone, Serialize, Deserialize, Type, PartialEq, Eq)]
pub enum QualityStage {
    #[sqlx(rename = "R&D")]
    Rd,             // 研发测试
    Production,      // 生产制造
    AfterSales,      // 售后维保
}

/// 严重程度
#[derive(Debug, Clone, Serialize, Deserialize, Type, PartialEq, Eq)]
#[sqlx(rename_all = "PascalCase")]
pub enum QualitySeverity {
    Blocker,
    Critical,
    Major,
    Minor,
}

/// 质量记录/缺陷状态流转
#[derive(Debug, Clone, Serialize, Deserialize, Type, PartialEq, Eq)]
#[sqlx(rename_all = "PascalCase")]
pub enum QualityStatus {
    Open,           // 开启/待处理
    InAnalysis,     // 原因分析中
    Fixed,          // 已修复/已处理
    Verified,       // 已验证
    Closed,         // 已关闭
    Ignored,        // 已忽略/暂不处理
}

// --- 2. 核心质量模型 ---

/// 全链路质量事件：记录从研发 Bug 到售后维修的所有单据
#[derive(Debug, Clone, FromRow, Serialize, Deserialize)]
pub struct QualityRecord {
    pub id: String,
    pub model_id: String,
    pub project_id: Option<String>,     // 研发阶段关联项目
    pub batch_no: Option<String>,       // 生产批次或固件版本
    
    pub stage: QualityStage,            // 阶段强约束
    pub issue_type: String,             // 分类 (可进一步枚举或使用文本)
    pub severity: QualitySeverity,      // 严重程度
    
    pub title: String,
    pub detail: Option<String>,
    pub root_cause: Option<String>,     // 根因分析 (RCA)
    pub solution: Option<String>,       // 解决方案
    
    pub status: QualityStatus,          // 处理状态
    
    // 审计字段
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub created_by: Option<String>,     // 提报人ID
    pub updated_by: Option<String>,     // 最后处理人ID
}

/// 质量度量快照：用于 TUI Dashboard 统计展示
#[derive(Debug, Clone, FromRow, Serialize, Deserialize)]
pub struct QualityMetricsSnapshot {
    pub id: String,
    pub model_id: String,
    pub snapshot_date: DateTime<Utc>,
    
    // 研发指标 (INTEGER -> i64, REAL -> Decimal)
    pub rd_bug_count: i64,
    pub test_coverage: Decimal,
    
    // 生产指标
    pub fpy_rate: Decimal,              // 直通率
    pub batch_fail_rate: Decimal,       // 批次不合格率
    
    // 售后指标
    pub return_rate: Decimal,           // 返修率
    pub mtbf_hours: i64,                // 平均无故障时间
    
    pub created_at: DateTime<Utc>,
    pub created_by: Option<String>,
}

// --- 3. 业务聚合模型 (DTO) ---

/// 质量趋势分析包
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QualityTrend {
    pub date_axis: Vec<DateTime<Utc>>,
    pub fpy_values: Vec<Decimal>,
    pub rd_bug_values: Vec<i64>,
}