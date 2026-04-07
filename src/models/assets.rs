use serde::{Deserialize, Serialize};
use sqlx::{FromRow, Type};
use chrono::{DateTime, Utc};

// --- 1. 自动化策略相关枚举 ---

/// 自动化适合度评估：决定是否投入研发资源
#[derive(Debug, Clone, Serialize, Deserialize, Type, PartialEq, Eq)]
#[sqlx(rename_all = "PascalCase")]
pub enum AutoSuitability {
    Suitable,       // 适合自动化 (ROI高)
    LowPriority,    // 收益不高，低优先级
    Impossible,     // 由于物理或技术限制无法实现
    Pending,        // 待评估
}

/// 不可自动化原因标准：用于指导 DFT (可测试性设计) 改进
#[derive(Debug, Clone, Serialize, Deserialize, Type, PartialEq, Eq)]
#[sqlx(rename_all = "PascalCase")]
pub enum NonAutoReason {
    PhysicalIntervention, // 需人工物理操作 (如插拔、翻转)
    HighMaintenance,      // 业务变动极快，脚本维护成本过高
    HardwareLimitation,   // 硬件未预留调试口或无法回传状态
    EnvironmentUnstable,  // 外部依赖环境 (如卫星信号) 无法控制
    None,                 // 适合自动化/无限制
}

// --- 2. 核心资产模型 ---

/// 测试用例：与产品型号及版本深度绑定的测试资产
#[derive(Debug, Clone, FromRow, Serialize, Deserialize)]
pub struct TestCase {
    pub id: String,
    pub base_case_id: String,     // 溯源ID，用于查看一个用例在各版本的变迁
    
    // 产品层级关联
    pub product_line_id: String,
    pub model_id: String,
    pub version_tag: String,      // 适用版本 (如: v1.0.2)
    
    pub title: String,
    pub precondition: Option<String>,
    pub steps: String,            // 测试步骤 (推荐 Markdown)
    pub expected_result: String,
    
    // 自动化策略核心
    pub auto_suitability: AutoSuitability,
    pub is_automated: bool,       // 当前版本是否已完成脚本编写
    pub non_auto_reason: Option<NonAutoReason>,
    pub automation_ref: Option<String>, // 指向 Rust 脚本路径或 ID
    
    // 审计字段
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub created_by: Option<String>,
    pub updated_by: Option<String>,
}

// --- 3. 统计模型 (对应视图结果) ---

/// 自动化 ROI 统计结果：用于 Dashboard 汇报
#[derive(Debug, Clone, FromRow, Serialize, Deserialize)]
pub struct AutomationRoi {
    pub model_id: String,
    pub version_tag: String,
    pub total_cases: i64,
    pub sl_cases: i64,             // 评估为“适合”的用例总数
    pub done_cases: i64,           // 实际已完成自动化的用例数
    pub automation_rate: f64,      // 针对“适合”部分的覆盖率
}