# Fuma RS

一个用 Rust 编写的高性能课程数据处理工具，用于替代原有的 Python + CLI 架构。

## 前置条件

本工具需要以下数据已经准备好：

1. **repos 目录**：包含所有课程的 `.mdx` 和 `.json` 文件
2. **培养方案数据**：`hoa-majors/src/hoa_majors/data/plans/*.toml` 文件
3. **repos_list.txt**（可选）：用于过滤需要处理的课程

如果 `repos` 目录不存在，工具会提示错误并退出。您需要先运行 Python 脚本来下载这些数据。

## 功能特性

- **高性能**：避免了 N+1 查询问题，一次性加载所有培养方案数据
- **单一二进制**：无需依赖外部 CLI 工具，可直接在 GitHub Actions 中运行
- **并发处理**：使用 Rust 异步运行时高效处理大量课程数据
- **零配置**：自动发现项目根目录，智能处理缺失文件

## 构建

```bash
cargo build --release
```

构建后的二进制文件位于 `target/release/fuma`

## 使用

### 方式一：直接运行二进制

从项目根目录运行：

```bash
./fuma_rs/target/release/fuma
```

### 方式二：使用 cargo run

```bash
cd fuma_rs && cargo run --release
```

### 完整工作流

如果您是首次使用，建议按以下顺序操作：

1. **运行 Python 脚本获取数据**（这部分保留原有流程）：
   ```bash
   cd scripts
   python main.py  # 下载 repos 数据
   ```

2. **运行 Rust 工具生成页面**：
   ```bash
   cd fuma_rs
   cargo run --release
   ```

## 工作流程

1. **加载培养方案**：从 `hoa-majors/src/hoa_majors/data/plans/*.toml` 读取所有培养方案
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

## 目录结构

```
hoa-fuma/
├── fuma_rs/                        # Rust 工具（本项目）
├── repos/                          # 课程资源（由 Python 脚本生成）
│   ├── COMP2001.mdx
│   ├── COMP2001.json
│   └── ...
├── hoa-majors/                     # 培养方案数据
│   └── src/hoa_majors/data/plans/
│       └── *.toml
├── content/docs/                   # 输出目录
└── repos_list.txt                  # 可选的课程过滤列表
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

## 性能优化

相比原有 Python + subprocess 架构的优化：

1. **避免进程创建开销**：不再为每个查询创建新进程
2. **批量数据加载**：一次性加载所有 TOML 文件到内存
3. **内存中数据结构**：使用 HashMap 快速查找，避免重复文件 I/O
4. **编译型语言**：Rust 的零成本抽象和编译优化

## 错误处理

使用 `thiserror` 提供清晰的错误信息：

- `Io`: 文件系统操作错误
- `Toml`: TOML 解析错误  
- `Json`: JSON 解析错误
- `MissingDataDir`: 数据目录缺失

## GitHub Actions 集成

完整的 workflow 示例：

```yaml
- name: Setup Rust
  uses: actions-rs/toolchain@v1
  with:
    toolchain: stable

- name: Fetch course data (Python part)
  run: |
    cd scripts
    python main.py
  env:
    GITHUB_TOKEN: ${{ secrets.GITHUB_TOKEN }}

- name: Build fuma
  run: |
    cd fuma_rs
    cargo build --release

- name: Generate course pages
  run: ./fuma_rs/target/release/fuma
```

或者使用 `cargo run`：

```yaml
- name: Generate pages
  run: cd fuma_rs && cargo run --release
```

## 与原有架构的对比

### 原有架构（Python）
- 使用 subprocess 调用 `hoa` CLI
- 每个查询创建新进程
- N+1 查询问题
- 需要安装 Python 依赖

### 新架构（Rust）
- 单一二进制，无外部依赖
- 一次性加载所有数据
- 批量处理，避免 N+1 问题
- 更快的执行速度
- 更少的内存占用

## 开发说明

如果需要修改代码，请注意：

1. 保持与原有 Python 输出的兼容性
2. YAML frontmatter 格式必须匹配
3. 文件树 JSX 结构必须保持一致
4. 索引页面的 Card 链接格式必须正确
