-- 1. 权限原子项表
CREATE TABLE IF NOT EXISTS permissions (
    id              TEXT PRIMARY KEY,
    code            TEXT NOT NULL UNIQUE,       -- 权限编码 (如 task:cancel, biz:view_profit)
    description     TEXT,                       -- 权限功能描述
    created_at      DATETIME DEFAULT CURRENT_TIMESTAMP,
    updated_at      DATETIME DEFAULT CURRENT_TIMESTAMP,
    created_by      TEXT,                       -- 创建人ID
    updated_by      TEXT                        -- 最后修改人ID
);

-- 2. 角色定义表
CREATE TABLE IF NOT EXISTS roles (
    id              TEXT PRIMARY KEY,
    name            TEXT NOT NULL UNIQUE CHECK (name IN ('ADMIN', 'PM', 'MEMBER')),
    description     TEXT,
    created_at      DATETIME DEFAULT CURRENT_TIMESTAMP,
    updated_at      DATETIME DEFAULT CURRENT_TIMESTAMP,
    created_by      TEXT,
    updated_by      TEXT
);

-- 3. 角色-权限关联表
CREATE TABLE IF NOT EXISTS role_permissions (
    role_id         TEXT NOT NULL,
    permission_id   TEXT NOT NULL,
    PRIMARY KEY (role_id, permission_id),
    FOREIGN KEY (role_id) REFERENCES roles(id) ON DELETE CASCADE,
    FOREIGN KEY (permission_id) REFERENCES permissions(id) ON DELETE CASCADE
);

-- 4. 增强版员工画像表
CREATE TABLE IF NOT EXISTS users (
    id              TEXT PRIMARY KEY,
    username        TEXT NOT NULL UNIQUE,       -- 登录账号
    nickname        TEXT,                       -- 昵称
    avatar_url      TEXT,                       -- 头像路径/URL
    email           TEXT UNIQUE,
    phone           TEXT UNIQUE,
    gender          TEXT CHECK (gender IN ('Male', 'Female', 'Other', 'Unknown')),
    constellation   TEXT,                       -- 星座
    current_city    TEXT,                       -- 常住地
    hometown        TEXT,                       -- 家乡
    tags            TEXT,                       -- 个人标签 (JSON Array: ["Rust", "QA"])
    password_hash   TEXT,                       -- 密码哈希
    created_at      DATETIME DEFAULT CURRENT_TIMESTAMP,
    updated_at      DATETIME DEFAULT CURRENT_TIMESTAMP,
    created_by      TEXT,
    updated_by      TEXT
);

-- 5. 用户-角色关联表
CREATE TABLE IF NOT EXISTS user_roles (
    user_id         TEXT NOT NULL,
    role_id         TEXT NOT NULL,
    PRIMARY KEY (user_id, role_id),
    FOREIGN KEY (user_id) REFERENCES users(id) ON DELETE CASCADE,
    FOREIGN KEY (role_id) REFERENCES roles(id) ON DELETE CASCADE
);

-- 自动更新用户表时间戳
CREATE TRIGGER IF NOT EXISTS trg_users_updated_at 
AFTER UPDATE ON users
BEGIN
    UPDATE users SET updated_at = CURRENT_TIMESTAMP WHERE id = OLD.id;
END;

-- 自动更新权限表时间戳
CREATE TRIGGER IF NOT EXISTS trg_permissions_updated_at 
AFTER UPDATE ON permissions
BEGIN
    UPDATE permissions SET updated_at = CURRENT_TIMESTAMP WHERE id = OLD.id;
END;