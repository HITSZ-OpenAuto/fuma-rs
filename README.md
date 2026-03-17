# HOA Backend

一个用 Rust 编写的高性能课程数据处理工具。

## 构建

```bash
cargo build --release
```

构建后的二进制文件位于 `target/release/hoa-backend`

## 发布（Release）注意事项

- GitHub Actions 的 `release` 只会在 **推送 tag** 时触发：`.github/workflows/build.yaml` 中有条件 `startsWith(github.ref, 'refs/tags/')`。
- 推荐发布流程（确保 tag 指向包含版本号更新的 commit）：
   1. 修改 `Cargo.toml` 的 `version`（如有生成/变更也一并更新 `Cargo.lock`）。
   2. 提交变更：`git add Cargo.toml Cargo.lock` → `git commit -m "chore: bump version to 1.15.1"`。
   3. 创建 **annotated tag**（使用 `v` 前缀，格式为 `v<大版本>.<小版本>.<修订版本>`，和 GitHub Release 命名规范一致）：`git tag -a v1.15.1 -m "v1.15.1"`。
   4. 推送到远端以触发流水线：`git push` 后再 `git push origin v1.15.1`（或使用 `git push --follow-tags`）。
- Release 显示为 **Draft** 是预期行为：工作流里配置了 `draft: true`，需要到 GitHub 的 Releases 页面手动点 **Publish release**；如需自动发布，把 `draft` 改为 `false` 或删除该行。
- 若 `build` job 失败，`release` 不会执行（`needs: [build]`）。
- `release` 步骤开启了 `fail_on_unmatched_files: true`：如果上传的 artifacts 名称/路径与配置不一致（例如 `hoa-backend-linux.tar.gz/hoa-backend-linux.tar.gz`），发布会直接失败。


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
