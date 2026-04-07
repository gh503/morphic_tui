-- 1. 测试用例库
CREATE TABLE IF NOT EXISTS test_cases (
    id              TEXT PRIMARY KEY,
    base_case_id    TEXT NOT NULL,              -- 跨版本追踪 ID
    
    -- 产品关联 (Hierarchy)
    product_line_id TEXT NOT NULL,
    model_id        TEXT NOT NULL,
    version_tag     TEXT NOT NULL,              -- 适用的版本号 (如: v1.0.2)
    
    title           TEXT NOT NULL,
    precondition    TEXT,
    steps           TEXT NOT NULL,
    expected_result TEXT NOT NULL,
    
    -- 自动化策略 (SDET 核心字段)
    auto_suitability TEXT NOT NULL CHECK (
        auto_suitability IN ('Suitable', 'LowPriority', 'Impossible', 'Pending')
    ), -- 评估：适合、优先级低、无法实现、待评估
    
    is_automated    BOOLEAN DEFAULT FALSE,      -- 是否已完成自动化实现
    
    -- 不可自动化标准/原因说明
    non_auto_reason  TEXT CHECK (
        non_auto_reason IN ('PhysicalIntervention', 'HighMaintenance', 'HardwareLimitation', 'EnvironmentUnstable', 'None')
    ), -- 原因：需物理人工干预、维护成本过高、硬件限制无法回传、环境不稳定、无（即可自动）
    
    automation_ref  TEXT,                       -- 自动化代码路径/函数名
    
    -- 审计字段
    created_at      DATETIME DEFAULT CURRENT_TIMESTAMP,
    updated_at      DATETIME DEFAULT CURRENT_TIMESTAMP,
    created_by      TEXT,
    updated_by      TEXT,
    
    FOREIGN KEY (product_line_id) REFERENCES product_lines(id),
    FOREIGN KEY (model_id) REFERENCES product_models(id),
    FOREIGN KEY (created_by) REFERENCES users(id)
);

-- 2. 自动化评估视图 (用于 ROI 分析)
CREATE VIEW IF NOT EXISTS view_automation_roi AS
SELECT 
    model_id,
    version_tag,
    COUNT(*) as total_cases,
    SUM(CASE WHEN auto_suitability = 'Suitable' THEN 1 ELSE 0 END) as sl_cases,
    SUM(CASE WHEN is_automated = 1 THEN 1 ELSE 0 END) as done_cases,
    ROUND(CAST(SUM(CASE WHEN is_automated = 1 THEN 1 ELSE 0 END) AS REAL) / 
          NULLIF(SUM(CASE WHEN auto_suitability = 'Suitable' THEN 1 ELSE 0 END), 0) * 100, 2) as automation_rate
FROM test_cases
GROUP BY model_id, version_tag;