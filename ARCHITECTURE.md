# Fuma RS 架构文档

## 概述

Fuma RS 是一个用 Rust 编写的高性能课程数据处理工具，用于替代原有的 Python + subprocess + CLI 架构。该工具从培养方案 TOML 文件中读取数据，生成结构化的课程页面和索引文件，并对 MDX 文件进行后处理。

## 设计目标

1. **消除 N+1 查询问题**：原架构中每次查询都需要创建子进程调用 CLI，导致大量重复的进程创建和数据加载
2. **提高性能**：使用 Rust 的零成本抽象和编译优化
3. **简化部署**：生成单一二进制文件，无需 Python 环境和依赖
4. **保持兼容性**：输出格式与原 Python 版本完全兼容

## 模块架构

```
fuma_rs/
├── src/
│   ├── main.rs           # 主入口，协调各模块
│   ├── error.rs          # 错误类型定义
│   ├── models.rs         # 数据模型
│   ├── constants.rs      # 常量和配置
│   ├── loader.rs         # 数据加载（TOML/repos_list）
│   ├── tree.rs           # 文件树生成和 JSX 转换
│   ├── generator.rs      # 页面生成逻辑
│   └── formatter.rs      # MDX 后处理和格式化
├── Cargo.toml            # 依赖配置
└── README.md             # 使用说明
```

### 模块职责

#### `main.rs`
- 程序入口点
- 协调各模块的执行流程
- 处理命令行参数（目前无参数，但保留扩展性）
- 错误处理和用户友好的输出

#### `error.rs`
- 使用 `thiserror` 定义统一的错误类型
- 错误类型：
  - `Io`: 文件系统操作错误
  - `Toml`: TOML 解析错误
  - `Json`: JSON 解析错误
  - `MissingDirectory`: 必需目录缺失

#### `models.rs`
- 定义所有数据结构
- 分为三类：
  1. **TOML 数据模型**：用于反序列化 TOML 文件
  2. **运行时模型**：程序内部使用的简化结构
  3. **文件树模型**：表示 GitHub 仓库的文件结构

#### `constants.rs`
- 学期映射表（中文名称 → 文件夹名 + 显示名）
- 文件排除规则（README.md, .gitkeep 等）
- 辅助函数：`get_semester_folder()`, `should_include_file()`

#### `loader.rs`
- `load_all_plans()`: 一次性加载所有 TOML 培养方案
  - 遍历 `plans/` 目录
  - 解析 TOML 文件
  - 转换为运行时模型
  - **关键优化**：避免 N+1 查询，所有数据一次性加载到内存
- `load_repos_list()`: 加载课程过滤列表（可选）

#### `tree.rs`
- `build_file_tree()`: 从扁平的 worktree.json 构建嵌套树结构
  - 使用 `TreeBuilder` 辅助结构递归构建
  - 自动排序：文件夹优先，然后按名称排序
  - 过滤排除的文件和目录
- `tree_to_jsx()`: 将树结构转换为 Fumadocs Files 组件的 JSX
  - 递归生成嵌套的 `<Folder>` 和 `<File>` 标签
  - 包含单元测试

#### `generator.rs`
- `generate_course_pages()`: 核心页面生成函数
  - 为每个培养方案生成目录结构
  - 按学期组织课程页面
  - 生成三级索引：年级 → 专业 → 学期
- `build_frontmatter()`: 生成 YAML frontmatter
  - 简化版本，只包含 Fumadocs 必需的字段
  - 自动处理特殊字符的引号

#### `formatter.rs`
- MDX 后处理，替代原 Python 的 `format_mdx.py`
- 转换规则：
  1. 移除 HTML 注释
  2. 移除 shield.io 徽章
  3. 修复自闭合标签（`<br>` → `<br />`）
  4. 修复畸形 HTML
  5. 转换 CSS style 为 JSX 格式（`style="text-align:center"` → `style={{textAlign: "center"}}`）
  6. 转义 LaTeX 数学公式中的花括号
  7. 转换 Hugo shortcode 为 Accordion 组件
- `format_all_mdx_files()`: 批量处理所有 MDX 文件
- 包含完整的单元测试套件

## 数据流

```
┌─────────────────────┐
│  hoa-majors TOML    │
│  培养方案数据       │
└──────────┬──────────┘
           │
           ▼
     ┌─────────┐
     │ Loader  │ ◄──── repos_list.txt (optional)
     └────┬────┘
          │
          ▼
     ┌─────────┐
     │  Plans  │ (in-memory)
     └────┬────┘
          │
          ▼
     ┌──────────┐
     │ Generator│ ◄──── repos/*.mdx, repos/*.json
     └────┬─────┘
          │
          ▼
   ┌──────────────┐
   │ content/docs/│
   │  - 2022/     │
   │  - 2023/     │
   │  - ...       │
   └──────┬───────┘
          │
          ▼
    ┌───────────┐
    │ Formatter │
    └─────┬─────┘
          │
          ▼
   ┌──────────────┐
   │ Formatted    │
   │ MDX files    │
   └──────────────┘
```

## 性能优化

### 1. 批量数据加载
- **原架构**：为每个 plan 调用一次 `hoa plans`，为每个 course 调用一次 `hoa courses`
- **新架构**：一次性加载所有 TOML 文件到内存，O(1) 查询复杂度

### 2. 零进程创建
- **原架构**：每次查询创建新进程（fork + exec + stdout 解析）
- **新架构**：纯内存操作，无进程创建开销

### 3. 编译优化
- Rust 编译器优化（`--release`）
- 零成本抽象
- 静态分发（无虚函数调用）

### 4. 并发处理
- 使用 Tokio 异步运行时
- 保留并发扩展点（当前为顺序处理，但架构支持并发）

## 输出兼容性

### YAML Frontmatter
```yaml
---
title: "课程名称"
description: ""
course:
  credit: 3
  assessmentMethod: "考试"
  courseNature: "必修"
  hourDistribution:
    theory: 48
    lab: 0
    practice: 0
    exercise: 0
    computer: 0
    tutoring: 0
  gradingScheme: []
---
```

### 文件树 JSX
```jsx
<Files url="https://github.com/HITSZ-OpenAuto/COMP2001">
  <Folder name="assignments">
    <File name="hw1.pdf" url="..." date="2024-01-01" size={1024} />
  </Folder>
  <File name="syllabus.pdf" url="..." date="2024-01-01" size={2048} />
</Files>
```

### 目录结构
```
content/docs/
├── 2022/
│   ├── meta.json          # {"title": "2022"}
│   ├── index.mdx          # 年级索引（Cards）
│   └── 010101/            # 专业代码
│       ├── meta.json      # {"title": "计算机科学与技术", "root": true, "defaultOpen": true}
│       ├── index.mdx      # 专业索引（学期 Cards）
│       ├── fresh-autumn/
│       │   ├── index.mdx  # 学期索引（课程 Cards）
│       │   └── COMP2001.mdx
│       └── ...
```

## 错误处理策略

1. **早期失败**：在 main 函数中检查必需目录
2. **清晰错误信息**：使用 thiserror 提供上下文
3. **优雅降级**：repos_list.txt 可选，缺失时处理所有课程
4. **跳过而非失败**：个别课程文件缺失时继续处理其他课程

## 测试策略

### 单元测试
- `tree.rs`: 文件树构建和排除规则
- `formatter.rs`: MDX 转换规则

### 集成测试（未来）
- 完整的端到端测试
- 快照测试（比较生成的 MDX 与预期输出）

## 扩展点

### 1. 并发处理
当前架构支持轻松添加并发：
```rust
// 在 generator.rs 中使用 tokio::spawn
for plan in plans {
    tokio::spawn(async move {
        process_plan(plan).await
    });
}
```

### 2. 增量更新
可以添加文件哈希缓存，只处理变更的课程

### 3. 配置文件
可以添加 `fuma.toml` 配置文件支持自定义路径等

### 4. 插件系统
通过 trait 定义扩展点，支持自定义格式化规则

## 依赖选择理由

- **tokio**: 标准的异步运行时，生态成熟
- **serde**: Rust 事实上的序列化标准
- **toml**: 官方推荐的 TOML 解析器
- **thiserror**: 符合 Rust 习惯的错误定义
- **walkdir**: 高效的目录遍历
- **regex**: 功能完整的正则表达式库
- **chrono**: 时间处理标准库
- **urlencoding**: 正确处理 URL 中的中文字符

## 与原 Python 架构对比

| 特性 | Python 版本 | Rust 版本 |
|------|------------|-----------|
| 执行方式 | subprocess 调用 CLI | 直接读取 TOML |
| 查询复杂度 | O(N*M) N+1 问题 | O(N) 批量加载 |
| 进程创建 | 数千次 | 0 次 |
| 内存占用 | 高（Python + 多进程） | 低（单进程） |
| 启动时间 | 慢（Python 解释器） | 快（编译二进制） |
| 依赖管理 | pip/uv + HOA CLI | 无运行时依赖 |
| 部署复杂度 | 需要 Python 环境 | 单一二进制 |

## 未来改进

1. **并行化**：利用多核 CPU 并行处理课程
2. **增量构建**：只重新生成变更的文件
3. **进度显示**：添加进度条（使用 indicatif）
4. **配置文件**：支持自定义路径和规则
5. **更多测试**：扩展测试覆盖率
6. **CI/CD 集成**：提供预编译的二进制文件下载