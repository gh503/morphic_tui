-- 1. 验收项定义表 (预设硬件/软件的验收标准)
CREATE TABLE IF NOT EXISTS acceptance_items (
    id              TEXT PRIMARY KEY,
    base_item_id    TEXT NOT NULL,              -- 溯源ID：用于追踪同一个指标（如“续航测试”）在不同版本间的变化
    category        TEXT NOT NULL CHECK (category IN ('Hardware', 'Software', 'System', 'Compliance')),
    version_tag     TEXT NOT NULL,              -- 标准的版本号 (如: "STD-2026-Q1", "STD-2026-Q2")
    
    title           TEXT NOT NULL,              -- 验收项名称
    standard_desc   TEXT,                       -- 具体的标准值 (旧版可能是 ">90%", 新版演进为 ">95%")
    is_critical     BOOLEAN DEFAULT FALSE,      -- 是否一票否决
    
    is_active       BOOLEAN DEFAULT TRUE,       -- 是否为当前生效的最新标准
    
    created_at      DATETIME DEFAULT CURRENT_TIMESTAMP,
    updated_at      DATETIME DEFAULT CURRENT_TIMESTAMP,
    created_by      TEXT,
    updated_by      TEXT
);

-- 2. 验收批次表 (记录每次针对特定版本/型号的验收工作)
CREATE TABLE IF NOT EXISTS acceptance_tasks (
    id              TEXT PRIMARY KEY,
    model_id        TEXT NOT NULL,
    version_name    TEXT NOT NULL,              -- 固件版本或软件版本 (如: V1.0.2_Build0406)
    
    -- 验收类型
    type            TEXT CHECK (type IN ('Alpha', 'Beta', 'RC', 'GA')), -- 内部/公测/发布候选/正式发售
    
    -- 评估结论
    conclusion      TEXT DEFAULT 'Pending' CHECK (
        conclusion IN ('Pending', 'Pass', 'ConditionalPass', 'Fail')
    ),
    summary         TEXT,                       -- 验收总结报告
    
    created_at      DATETIME DEFAULT CURRENT_TIMESTAMP,
    updated_at      DATETIME DEFAULT CURRENT_TIMESTAMP,
    created_by      TEXT,                       -- 验收负责人
    updated_by      TEXT,
    FOREIGN KEY (model_id) REFERENCES product_models(id)
);

-- 3. 验收明细记录表 (具体的执行记录)
CREATE TABLE IF NOT EXISTS acceptance_records (
    id              TEXT PRIMARY KEY,
    task_id         TEXT NOT NULL,
    item_id         TEXT NOT NULL,
    
    result          TEXT CHECK (result IN ('Pass', 'Fail', 'N/A')),
    measured_value  TEXT,                       -- 实际测量值/观测结果
    remark          TEXT,                       -- 备注/风险说明
    evidence_url    TEXT,                       -- 测试截图或报告链接
    
    updated_at      DATETIME DEFAULT CURRENT_TIMESTAMP,
    updated_by      TEXT,
    FOREIGN KEY (task_id) REFERENCES acceptance_tasks(id),
    FOREIGN KEY (item_id) REFERENCES acceptance_items(id)
);

-- 4. 统计评估视图 (用于版本演进对比)
-- 记录每个版本的 验收通过率、关键风险项数量 等
CREATE TABLE IF NOT EXISTS acceptance_evaluation (
    id              TEXT PRIMARY KEY,
    task_id         TEXT NOT NULL,
    pass_rate       REAL,                       -- 本次验收通过率
    critical_fails  INTEGER DEFAULT 0,          -- 核心项失败数
    trend           TEXT,                       -- 相比上个版本的演进趋势说明
    
    created_at      DATETIME DEFAULT CURRENT_TIMESTAMP,
    FOREIGN KEY (task_id) REFERENCES acceptance_tasks(id)
);

-- 5. 触发器
CREATE TRIGGER IF NOT EXISTS trg_acceptance_tasks_updated
AFTER UPDATE ON acceptance_tasks
BEGIN
    UPDATE acceptance_tasks SET updated_at = CURRENT_TIMESTAMP WHERE id = OLD.id;
END;