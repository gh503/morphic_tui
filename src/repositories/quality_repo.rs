use sqlx::{SqlitePool, Row};
use crate::apps::quality::{TaskRecord, BugRecord};
use crate::models::*;
use std::sync::Arc;
use anyhow::{Result, Context};

pub struct QualityRepository {
    pool: Arc<SqlitePool>,
}

impl QualityRepository {
    pub fn new(pool: Arc<SqlitePool>) -> Self {
        Self { pool }
    }

    pub async fn fetch_projects(&self) -> Result<Vec<Project>> {
        let projects = sqlx::query_as::<_, Project>("SELECT * FROM projects")
            .fetch_all(&*self.pool)
            .await
            .context("Failed to fetch projects")?;
        Ok(projects)
    }

    pub async fn fetch_tasks(&self) -> Result<Vec<TaskRecord>> {
        let rows = sqlx::query("SELECT title, status, priority FROM tasks")
            .fetch_all(&*self.pool)
            .await?;
        Ok(rows.iter().map(|r| TaskRecord {
            title: r.get("title"),
            status: r.get("status"),
            priority: r.get("priority"),
        }).collect())
    }

    pub async fn fetch_bugs(&self) -> Result<Vec<BugRecord>> {
        let rows = sqlx::query("SELECT id, title, severity, status FROM bugs")
            .fetch_all(&*self.pool)
            .await?;
        Ok(rows.iter().map(|r| BugRecord {
            id: r.get("id"),
            title: r.get("title"),
            severity: r.get("severity"),
            status: r.get("status"),
        }).collect())
    }

    // 修复：明确表名和列名
    pub async fn fetch_acceptance(&self) -> Result<Vec<String>> {
        let rows = sqlx::query("SELECT criteria FROM acceptance_criteria")
            .fetch_all(&*self.pool)
            .await
            .context("Check if table 'acceptance_criteria' exists")?;
        Ok(rows.iter().map(|r| r.get::<String, _>("criteria")).collect())
    }

    // 修复：明确表名
    pub async fn fetch_assets(&self) -> Result<Vec<(String, String)>> {
        let rows = sqlx::query("SELECT name, status FROM assets")
            .fetch_all(&*self.pool)
            .await
            .context("Check if table 'assets' exists")?;
        Ok(rows.iter().map(|r| (r.get("name"), r.get("status"))).collect())
    }

    pub async fn add_acceptance(&self, content: &str) -> Result<()> {
        sqlx::query("INSERT INTO acceptance_criteria (criteria) VALUES (?)")
            .bind(content)
            .execute(&*self.pool)
            .await?;
        Ok(())
    }
}