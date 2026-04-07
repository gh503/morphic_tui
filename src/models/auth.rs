use serde::{Deserialize, Serialize};
use sqlx::{FromRow, Type};
use chrono::{DateTime, Utc};

// --- 枚举与常量约束 ---

/// 角色类型约束：与 SQL 中的 CHECK 约束对齐
#[derive(Debug, Clone, Serialize, Deserialize, Type, PartialEq, Eq)]
#[sqlx(rename_all = "UPPERCASE")]
pub enum RoleName {
    Admin,
    Pm,
    Member,
}

/// 性别枚举
#[derive(Debug, Clone, Serialize, Deserialize, Type, PartialEq, Eq)]
#[sqlx(rename_all = "PascalCase")]
pub enum UserGender {
    Male,
    Female,
    Other,
    Unknown,
}

// --- 核心实体模型 ---

/// 权限原子项：定义系统功能权限
#[derive(Debug, Clone, FromRow, Serialize, Deserialize)]
pub struct Permission {
    pub id: String,
    pub code: String,               // 权限编码 (如 task:cancel)
    pub description: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub created_by: Option<String>,
    pub updated_by: Option<String>,
}

/// 角色定义
#[derive(Debug, Clone, FromRow, Serialize, Deserialize)]
pub struct Role {
    pub id: String,
    pub name: RoleName,             // 强类型角色
    pub description: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub created_by: Option<String>,
    pub updated_by: Option<String>,
}

/// 用户画像：包含丰富的员工背景信息
#[derive(Debug, Clone, FromRow, Serialize, Deserialize)]
pub struct User {
    pub id: String,
    pub username: String,
    pub nickname: Option<String>,
    pub avatar_url: Option<String>,
    pub email: Option<String>,
    pub phone: Option<String>,
    pub gender: Option<UserGender>,
    pub constellation: Option<String>,
    pub current_city: Option<String>,
    pub hometown: Option<String>,
    
    /// 个人标签：使用 sqlx::types::Json 自动处理数据库中的 TEXT(JSON) 到 Vec 的转换
    pub tags: Option<sqlx::types::Json<Vec<String>>>,
    
    #[serde(skip_serializing)]      // 序列化时隐藏密码哈希
    pub password_hash: Option<String>,
    
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub created_by: Option<String>,
    pub updated_by: Option<String>,
}

// --- 关联关系模型 ---

/// 角色-权限关联
#[derive(Debug, Clone, FromRow, Serialize, Deserialize)]
pub struct RolePermission {
    pub role_id: String,
    pub permission_id: String,
}

/// 用户-角色关联
#[derive(Debug, Clone, FromRow, Serialize, Deserialize)]
pub struct UserRole {
    pub user_id: String,
    pub role_id: String,
}

// --- 业务聚合模型 (DTO) ---

/// 登录后的用户信息上下文，包含其所有的权限集合
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserIdentity {
    pub user: User,
    pub roles: Vec<RoleName>,
    pub permissions: Vec<String>,   // 扁平化的权限 code 列表
}