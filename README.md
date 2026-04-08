# Morphic TUI

一个基于 Rust 和 Ratatui 构建的、面向 SDET 和质量工程的系统监控与管理终端工具。

## 🚀 核心特性 (Core Features)

* **📊 性能监控 (Performance Monitor)**：基于 `sysinfo` 的实时 CPU 与内存负载分析，支持可配置的采样深度和动态背景网格渲染。
* **⚙️ 动态配置架构 (Config-Driven Architecture)**：
    * **全组件同步**：全局 `AppConfig` 实时驱动各子应用布局。
    * **列定义引擎**：支持通过 `config.toml` 动态配置表格列（名称、宽度、可见性）。
* **🛡️ 质量视角 (Quality Insight)**：内置项目管理、缺陷跟踪和验收标准视图，助力质量负责人（Quality Lead）掌控全局。
* **🖱️ 交互式 UI (Interactive UI)**：支持侧边栏动画、鼠标拖拽调节宽度以及 Tab 快速切换。
* **⚡ 极致性能优化**：
    * **按需分发**：仅对活跃 Tab 进行事件分发，非活跃组件零 CPU 占用。
    * **缓存机制**：背景网格与硬件信息通过 `RefCell` 和 `Instant` 实现精细化缓存刷新。

## 🛠️ 配置说明 (Configuration)

配置文件通常位于 `~/.config/morphic_tui/default-config.toml`。

### 采样点配置
在“设置”页面调整采样点，系统会自动持久化并同步所有组件的渲染轴：
```toml
max_points = 100 # CPU 历史图表采样深度
```

### 表格列配置 (Table Columns)
你可以自定义任何表格的展示维度：
```toml
[table_columns]
projects = [
    { name = "项目名称", width = 60, visible = true, sort = "None" },
    { name = "状态", width = 40, visible = true, sort = "None" }
]
# 支持 tasks, bugs, acceptance, assets 等多个 Tab 定义
```

## ⌨️ 操作指南 (Keymap)

| 按键 | 功能 |
| :--- | :--- |
| `Tab` | 循环切换功能模块 (Monitor -> Settings -> Info -> Quality) |
| `b` | 展开/收起侧边栏 (带缓动动画) |
| `↑ / ↓` | (设置页) 动态调整 CPU 采样频率/深度 |
| `1 / 2` | (监控页) 按 CPU 或 内存对进程排序 |
| `f` | (质量页) 切换表头聚焦模式，开启排序逻辑 |
| `Shift + 鼠标` | 强制文本选区（绕过 TUI 捕获） |

## 🏗️ 架构愿景 (Architecture)



项目采用**单向数据流**设计：
1. **Event** 被 `RootApp` 捕获。
2. **Action** 由子组件反馈并统一在 `app.rs` 调度中心处理。
3. **State** 同步更新至全局 `AppConfig`。
4. **Render** 阶段所有组件共享配置快照，确保 UI 最终一致性。
