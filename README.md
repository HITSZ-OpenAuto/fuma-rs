# HOA Backend

一个用 Rust 编写的高性能课程数据处理工具。

## 构建

```bash
cargo build --release
```

构建后的二进制文件位于 `target/release/hoa-backend`


## 工作流程

1. **加载培养方案**：从 `hoa_major-data/plans/*.toml` 读取所有培养方案
2. **过滤课程**：根据 `repos_list.txt`（如果存在）过滤可用课程
3. **读取资源**：从 `repos/` 目录读取课程的 `.mdx` 和 `.json` 文件
4. **生成页面**：
   - 为每个课程生成 MDX 页面，包含 YAML frontmatter
   - 从 `worktree.json` 生成文件树 JSX
   - 根据学期自动分类课程
   - 生成学期索引、专业索引和年级索引

## 输出结构

```
content/docs/
├── 2022/
│   ├── meta.json
│   ├── index.mdx
│   └── 010101/                    # 专业代码
│       ├── meta.json
│       ├── index.mdx
│       ├── fresh-autumn/          # 大一秋季
│       │   ├── index.mdx
│       │   ├── COMP2001.mdx
│       │   └── ...
│       └── ...
└── ...
```

## 配置文件

### repos_list.txt（可选）

位于项目根目录，每行一个课程代码，用于过滤需要处理的课程。如果文件不存在，将处理所有课程。

示例：
```
COMP2001
COMP2003
MATH1001
```

## 依赖项

- `tokio`: 异步运行时
- `serde`: 序列化/反序列化
- `toml`: TOML 文件解析
- `serde_json`: JSON 处理
- `thiserror`: 错误处理
- `walkdir`: 目录遍历
- `urlencoding`: URL 编码
- `chrono`: 时间戳格式化
