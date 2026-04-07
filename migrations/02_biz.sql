-- 1. 产品线表
CREATE TABLE IF NOT EXISTS product_lines (
    id              TEXT PRIMARY KEY,
    name            TEXT NOT NULL UNIQUE,       -- 产品线名称 (如: AI硬件)
    manager_id      TEXT,                       -- 产品线负责人
    description     TEXT,
    created_at      DATETIME DEFAULT CURRENT_TIMESTAMP,
    updated_at      DATETIME DEFAULT CURRENT_TIMESTAMP,
    created_by      TEXT,
    updated_by      TEXT,
    FOREIGN KEY (manager_id) REFERENCES users(id)
);

-- 2. 产品型号表
CREATE TABLE IF NOT EXISTS product_models (
    id              TEXT PRIMARY KEY,
    product_line_id TEXT NOT NULL,
    model_code      TEXT NOT NULL UNIQUE,       -- 型号代码 (如: Qidodo-V1)
    status          TEXT NOT NULL CHECK (
        status IN ('Planning', 'Developing', 'Testing', 'Released', 'Maintained', 'EndOfLife')
    ),
    description     TEXT,
    created_at      DATETIME DEFAULT CURRENT_TIMESTAMP,
    updated_at      DATETIME DEFAULT CURRENT_TIMESTAMP,
    created_by      TEXT,
    updated_by      TEXT,
    FOREIGN KEY (product_line_id) REFERENCES product_lines(id)
);

-- 3. 项目表 (研发维度的载体)
CREATE TABLE IF NOT EXISTS projects (
    id              TEXT PRIMARY KEY,
    model_id        TEXT NOT NULL,              -- 归属型号
    name            TEXT NOT NULL,              -- 项目名称
    status          TEXT NOT NULL DEFAULT 'Active' CHECK (
        status IN ('Active', 'Archived', 'Completed', 'Suspended')
    ),
    created_at      DATETIME DEFAULT CURRENT_TIMESTAMP,
    updated_at      DATETIME DEFAULT CURRENT_TIMESTAMP,
    created_by      TEXT,
    updated_by      TEXT,
    FOREIGN KEY (model_id) REFERENCES product_models(id)
);

-- 4. 里程碑表 (项目任务的阶段锚点)
CREATE TABLE IF NOT EXISTS milestones (
    id              TEXT PRIMARY KEY,
    project_id      TEXT NOT NULL,
    title           TEXT NOT NULL,              -- 里程碑名称
    due_date        DATETIME NOT NULL,          -- 计划达成日期
    status          TEXT NOT NULL DEFAULT 'Planned' CHECK (
        status IN ('Planned', 'InProgress', 'Achieved', 'Delayed', 'Cancelled')
    ),
    created_at      DATETIME DEFAULT CURRENT_TIMESTAMP,
    updated_at      DATETIME DEFAULT CURRENT_TIMESTAMP,
    created_by      TEXT,
    updated_by      TEXT,
    FOREIGN KEY (project_id) REFERENCES projects(id)
);

-- 5. 多维经营指标表 (财务与生产快照)
CREATE TABLE IF NOT EXISTS product_metrics (
    id                  TEXT PRIMARY KEY,
    model_id            TEXT NOT NULL,
    rd_cost             REAL DEFAULT 0.0,       -- 研发成本投入
    production_count    INTEGER DEFAULT 0,      -- 累计产量
    unit_cost           REAL DEFAULT 0.0,       -- 单台物料成本
    sales_count         INTEGER DEFAULT 0,      -- 累计销量
    revenue             REAL DEFAULT 0.0,       -- 总营收
    profit              REAL DEFAULT 0.0,       -- 净利润
    maintenance_count   INTEGER DEFAULT 0,      -- 售后/报修数
    failure_rate        REAL DEFAULT 0.0,       -- 故障率
    snapshot_date       DATETIME DEFAULT CURRENT_TIMESTAMP, -- 数据统计截点
    created_at          DATETIME DEFAULT CURRENT_TIMESTAMP,
    created_by          TEXT,                   -- 数据录入人
    FOREIGN KEY (model_id) REFERENCES product_models(id)
);

-- 自动更新产品型号时间戳
CREATE TRIGGER IF NOT EXISTS trg_product_models_updated_at 
AFTER UPDATE ON product_models
BEGIN
    UPDATE product_models SET updated_at = CURRENT_TIMESTAMP WHERE id = OLD.id;
END;

-- 自动更新项目时间戳
CREATE TRIGGER IF NOT EXISTS trg_projects_updated_at 
AFTER UPDATE ON projects
BEGIN
    UPDATE projects SET updated_at = CURRENT_TIMESTAMP WHERE id = OLD.id;
END;

-- 自动更新里程碑时间戳
CREATE TRIGGER IF NOT EXISTS trg_milestones_updated_at 
AFTER UPDATE ON milestones
BEGIN
    UPDATE milestones SET updated_at = CURRENT_TIMESTAMP WHERE id = OLD.id;
END;