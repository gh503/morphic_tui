-- 1. 通知队列
CREATE TABLE IF NOT EXISTS notification_queue (
    id              TEXT PRIMARY KEY,
    recipient_id    TEXT NOT NULL,              -- 接收人
    task_id         TEXT,                       -- 触发通知的任务(可选)
    subject         TEXT NOT NULL,              -- 通知标题
    body            TEXT NOT NULL,              -- 通知正文内容
    notify_channels TEXT NOT NULL,              -- JSON数组: ["EMAIL", "SYSTEM", "WEBHOOK"]
    status          TEXT NOT NULL DEFAULT 'Pending' CHECK (
        status IN ('Pending', 'Sent', 'PartialFailed', 'Failed')
    ),
    retry_count     INTEGER DEFAULT 0,
    next_retry_at   DATETIME,
    created_at      DATETIME DEFAULT CURRENT_TIMESTAMP,
    created_by      TEXT,
    FOREIGN KEY (recipient_id) REFERENCES users(id),
    FOREIGN KEY (task_id) REFERENCES tasks(id)
);

-- 自动更新通知重试时间 (可选逻辑：当重试次数增加时，自动计算下次重试时间)
CREATE TRIGGER IF NOT EXISTS trg_notification_retry_logic
AFTER UPDATE OF retry_count ON notification_queue
BEGIN
    UPDATE notification_queue 
    SET next_retry_at = DATETIME(CURRENT_TIMESTAMP, '+' || (OLD.retry_count * 5) || ' minutes')
    WHERE id = OLD.id AND status = 'Pending';
END;