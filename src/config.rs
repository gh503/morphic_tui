// src/config.rs
use serde::{Serialize, Deserialize};
use std::collections::HashMap;

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub enum SortOrder {
    None,
    Asc,
    Desc,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ColumnConfig {
    pub name: String,
    pub visible: bool,
    pub width: u16, // 百分比
    pub sort: SortOrder,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct AppConfig {
    pub sidebar_width: u16,
    pub max_points: usize,
    pub show_sidebar: bool,
    // 新增：为每个 Tab 存储表格列配置
    // Key 可以是 "projects", "bugs", "assets" 等
    pub table_columns: HashMap<String, Vec<ColumnConfig>>,
}

impl Default for AppConfig {
    fn default() -> Self {
        let mut table_columns = std::collections::HashMap::new();

        // 1. Projects
        table_columns.insert("projects".to_string(), vec![
            ColumnConfig { name: "项目名称".into(), width: 60, visible: true, sort: SortOrder::None },
            ColumnConfig { name: "状态".into(), width: 40, visible: true, sort: SortOrder::None },
        ]);

        // 2. Tasks
        table_columns.insert("tasks".to_string(), vec![
            ColumnConfig { name: "任务标题".into(), width: 50, visible: true, sort: SortOrder::None },
            ColumnConfig { name: "状态".into(), width: 25, visible: true, sort: SortOrder::None },
            ColumnConfig { name: "优先级".into(), width: 25, visible: true, sort: SortOrder::None },
        ]);

        // 3. Bugs (质量)
        table_columns.insert("bugs".to_string(), vec![
            ColumnConfig { name: "ID".into(), width: 15, visible: true, sort: SortOrder::None },
            ColumnConfig { name: "缺陷描述".into(), width: 45, visible: true, sort: SortOrder::None },
            ColumnConfig { name: "级别".into(), width: 20, visible: true, sort: SortOrder::None },
            ColumnConfig { name: "状态".into(), width: 20, visible: true, sort: SortOrder::None },
        ]);

        // 4. Acceptance (验收)
        table_columns.insert("acceptance".to_string(), vec![
            ColumnConfig { name: "验收标准 (Criteria)".into(), width: 100, visible: true, sort: SortOrder::None },
        ]);

        // 5. Assets (资产)
        table_columns.insert("assets".to_string(), vec![
            ColumnConfig { name: "资产名称".into(), width: 60, visible: true, sort: SortOrder::None },
            ColumnConfig { name: "部署状态".into(), width: 40, visible: true, sort: SortOrder::None },
        ]);

        Self {
            sidebar_width: 25,
            max_points: 50,
            show_sidebar: true,
            table_columns,
        }
    }
}