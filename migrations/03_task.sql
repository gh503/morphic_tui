-- 1. 任务主表
CREATE TABLE IF NOT EXISTS tasks (
    id              TEXT PRIMARY KEY,
    milestone_id    TEXT NOT NULL,              -- 所属里程碑
    assignee_id     TEXT,                       -- 负责人
    title           TEXT NOT NULL,
    content         TEXT,                       -- 任务详细描述
    status          TEXT NOT NULL CHECK (
        status IN ('Todo', 'InProgress', 'Testing', 'Done', 'Blocked', 'Cancelled')
    ),
    priority        INTEGER DEFAULT 1 CHECK (priority BETWEEN 1 AND 3),   -- 1:低, 3:高
    risk_level      INTEGER DEFAULT 0 CHECK (risk_level BETWEEN 0 AND 3), -- 风险等级 (0:正常, 3:极高)
    progress        INTEGER DEFAULT 0 CHECK (progress BETWEEN 0 AND 100), -- 0-100进度
    deadline        DATETIME,                   -- 任务截止日期
    created_at      DATETIME DEFAULT CURRENT_TIMESTAMP,
    updated_at      DATETIME DEFAULT CURRENT_TIMESTAMP,
    created_by      TEXT,
    updated_by      TEXT,
    FOREIGN KEY (milestone_id) REFERENCES milestones(id),
    FOREIGN KEY (assignee_id) REFERENCES users(id)
);

-- 2. 任务更新/风险记录流水表 (审计追踪)
CREATE TABLE IF NOT EXISTS task_journals (
    id              TEXT PRIMARY KEY,
    task_id         TEXT NOT NULL,
    updater_id      TEXT NOT NULL,              -- 操作人
    prev_status     TEXT,                       -- 变更前状态
    curr_status     TEXT NOT NULL,              -- 变更后状态
    progress_delta  INTEGER NOT NULL,           -- 进度变动值
    content         TEXT,                       -- 阶段性成果说明或风险备注
    created_at      DATETIME DEFAULT CURRENT_TIMESTAMP,
    created_by      TEXT,                       -- 冗余字段用于统计
    FOREIGN KEY (task_id) REFERENCES tasks(id),
    FOREIGN KEY (updater_id) REFERENCES users(id)
);

-- 自动更新任务表时间戳
CREATE TRIGGER IF NOT EXISTS trg_tasks_updated_at 
AFTER UPDATE ON tasks
BEGIN
    UPDATE tasks SET updated_at = CURRENT_TIMESTAMP WHERE id = OLD.id;
END;