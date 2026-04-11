// src/config.rs
use serde::{Serialize, Deserialize};
use std::collections::HashMap;

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub enum SortOrder { None, Asc, Desc }

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ColumnConfig {
    pub name: String,
    pub visible: bool,
    pub width: u16, 
    pub sort: SortOrder,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct AppConfig {
    pub sidebar_width: u16,
    pub max_points: usize,
    pub show_sidebar: bool,
    pub table_columns: HashMap<String, Vec<ColumnConfig>>,
}

impl AppConfig {

    pub fn save(&self) -> anyhow::Result<()> {
        // confy::store 会自动寻找 ~/.config/morphic_tui/default-config.toml 并写入
        // 第一个参数是 app_name，第二个参数是 config_name (对应文件名)，第三个是数据
        confy::store("morphic_tui", "default-config", self)
            .map_err(|e| anyhow::anyhow!("Confy 保存失败: {}", e))?;
        Ok(())
    }

    pub fn validate(&mut self) {
        self.max_points = self.max_points.clamp(10, 500);
        self.sidebar_width = self.sidebar_width.clamp(0, 60);

        let default_config = Self::default();
        let essential_tabs = ["projects", "tasks", "bugs", "acceptance", "assets"];
        
        for tab in essential_tabs {
            if !self.table_columns.contains_key(tab) {
                if let Some(default_cols) = default_config.table_columns.get(tab) {
                    self.table_columns.insert(tab.to_string(), default_cols.clone());
                }
            }
        }

        // 宽度权重与可见性防线
        for columns in self.table_columns.values_mut() {
            if columns.is_empty() { continue; }
            // 防御：禁止所有列隐藏
            if columns.iter().all(|c| !c.visible) {
                columns[0].visible = true;
            }
            // 修正宽度，防止溢出 100%（可选，此处通过 clamp 确保合理性）
            for col in columns.iter_mut() {
                col.width = col.width.clamp(5, 100);
            }
        }
    }
}

impl Default for AppConfig {
    fn default() -> Self {
        let mut table_columns = HashMap::new();
        
        table_columns.insert("projects".into(), vec![
            ColumnConfig { name: "项目".into(), width: 60, visible: true, sort: SortOrder::None },
            ColumnConfig { name: "状态".into(), width: 40, visible: true, sort: SortOrder::None },
        ]);
        
        table_columns.insert("tasks".into(), vec![
            ColumnConfig { name: "任务标题".into(), width: 50, visible: true, sort: SortOrder::None },
            ColumnConfig { name: "状态".into(), width: 25, visible: true, sort: SortOrder::None },
            ColumnConfig { name: "优先级".into(), width: 25, visible: true, sort: SortOrder::None },
        ]);

        table_columns.insert("bugs".into(), vec![
            ColumnConfig { name: "ID".into(), width: 15, visible: true, sort: SortOrder::None },
            ColumnConfig { name: "标题".into(), width: 45, visible: true, sort: SortOrder::None },
            ColumnConfig { name: "级别".into(), width: 20, visible: true, sort: SortOrder::None },
            ColumnConfig { name: "状态".into(), width: 20, visible: true, sort: SortOrder::None },
        ]);

        Self {
            sidebar_width: 25,
            max_points: 50,
            show_sidebar: true,
            table_columns,
        }
    }
}