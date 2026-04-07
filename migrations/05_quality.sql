-- 1. 全链路质量事件表 (Quality Events)
-- 涵盖：研发阶段的Bug、生产阶段的缺陷、售卖后的报修
CREATE TABLE IF NOT EXISTS quality_records (
    id              TEXT PRIMARY KEY,
    model_id        TEXT NOT NULL,
    project_id      TEXT,                       -- 研发阶段必填，售后阶段可选
    batch_no        TEXT,                       -- 生产批次号或软件版本号
    
    -- 质量阶段约束
    stage           TEXT NOT NULL CHECK (
        stage IN ('R&D', 'Production', 'AfterSales')
    ),
    
    -- 问题分类 (如: 硬件故障, 软件崩溃, 算法异常, 外观受损)
    issue_type      TEXT NOT NULL,
    
    -- 严重程度
    severity        TEXT CHECK (severity IN ('Blocker', 'Critical', 'Major', 'Minor')),
    
    title           TEXT NOT NULL,              -- 简要描述
    detail          TEXT,                       -- 详细现象/复现步骤/日志
    root_cause      TEXT,                       -- 根本原因分析 (RCA)
    solution        TEXT,                       -- 解决方案/修复措施
    
    -- 状态流转
    status          TEXT NOT NULL DEFAULT 'Open' CHECK (
        status IN ('Open', 'InAnalysis', 'Fixed', 'Verified', 'Closed', 'Ignored')
    ),
    
    -- 审计与追踪字段
    created_at      DATETIME DEFAULT CURRENT_TIMESTAMP,
    updated_at      DATETIME DEFAULT CURRENT_TIMESTAMP,
    created_by      TEXT,                       -- 提报人 (测试、质检或客服)
    updated_by      TEXT,                       -- 最后处理人
    
    FOREIGN KEY (model_id)  REFERENCES product_models(id),
    FOREIGN KEY (project_id) REFERENCES projects(id),
    FOREIGN KEY (created_by) REFERENCES users(id)
);

-- 2. 质量度量快照 (Quality Metrics)
-- 用于生成 Dashboard，统计直通率、Bug密度、返修率
CREATE TABLE IF NOT EXISTS quality_metrics_snapshot (
    id              TEXT PRIMARY KEY,
    model_id        TEXT NOT NULL,
    snapshot_date   DATETIME DEFAULT CURRENT_TIMESTAMP,
    
    -- 研发指标
    rd_bug_count    INTEGER DEFAULT 0,          -- 研发未关闭缺陷数
    test_coverage   REAL DEFAULT 0.0,           -- 测试覆盖率
    
    -- 生产指标
    fpy_rate        REAL DEFAULT 0.0,           -- 生产直通率 (First Pass Yield)
    batch_fail_rate REAL DEFAULT 0.0,           -- 批次不合格率
    
    -- 售后指标
    return_rate     REAL DEFAULT 0.0,           -- 退货/返修率
    mtbf_hours      INTEGER DEFAULT 0,          -- 平均无故障时间
    
    created_at      DATETIME DEFAULT CURRENT_TIMESTAMP,
    created_by      TEXT,
    FOREIGN KEY (model_id) REFERENCES product_models(id)
);

-- 3. 辅助触发器
CREATE TRIGGER IF NOT EXISTS trg_quality_records_updated_at
AFTER UPDATE ON quality_records
BEGIN
    UPDATE quality_records SET updated_at = CURRENT_TIMESTAMP WHERE id = OLD.id;
END;