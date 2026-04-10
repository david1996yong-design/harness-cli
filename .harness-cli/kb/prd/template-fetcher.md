# 远程模板获取

> 从远程仓库下载 spec 模板到本地项目，并为 update 提供模板哈希追踪

## 模块概述

远程模板获取模块支持从 GitHub、GitLab、Bitbucket 以及自托管实例下载预定义的 spec 模板。init 命令的 `--template` 选项触发市场模板下载，`--registry` 选项支持自定义 registry。模块同时提供 HTTP 代理检测、凭证脱敏，以及基于 SHA256 的模板哈希追踪机制（供 update 命令检测用户修改）。

## 关键文件

| 文件路径 | 职责 |
|----------|------|
| `src/utils/template_fetcher.rs` | 模板索引获取、模板下载、registry 源解析、自定义 registry 探测和直接下载 |
| `src/utils/proxy.rs` | HTTP 代理环境变量检测（`setup_proxy`）和凭证脱敏（`mask_proxy_url`） |
| `src/utils/template_hash.rs` | SHA256 哈希计算、`.template-hashes.json` 加载/保存、修改检测、初始哈希生成 |

## 核心功能

### 模板索引获取

- **业务规则**: `fetch_template_index(url)` 从默认或自定义 URL 下载模板列表 JSON
- **触发条件**: init `--template` 或交互式选择模板时
- **处理流程**: HTTP GET 请求（5 秒超时）-> 解析 `TemplateIndex { version, templates }` -> 返回 `Vec<SpecTemplate>`

### 模板下载

- **业务规则**: `download_template_by_id(cwd, id, strategy, prefetched, registry, dest_dir)` 根据模板 ID 下载到指定目录
- **触发条件**: 用户指定 `--template` 或在交互菜单中选择时
- **处理流程**: 构造 giget-style 仓库 URL -> git clone 到临时目录 -> 根据 `TemplateStrategy`（Skip/Overwrite/Append）合并到目标 spec 目录
- **策略**:
  - `Skip`: 跳过已存在文件
  - `Overwrite`: 覆盖目标 spec 目录
  - `Append`: 只添加缺失文件

### Registry 源解析

- **业务规则**: `parse_registry_source(source)` 支持多种输入格式：
  - giget 前缀：`gh:org/repo`、`gitlab:org/repo`、`bitbucket:org/repo`
  - HTTPS URL：`https://github.com/user/repo[/tree/branch/path]`
  - SSH URL：`git@host:org/repo.git`、`ssh://git@host:port/org/repo.git`
  - 自托管实例（通过 host 字段）
- **处理流程**: 先用 `normalize_registry_source` 将 HTTPS URL 转为 giget 格式，再解析为 `RegistrySource { provider, repo, subdir, ref_, raw_base_url, giget_source, host }`

### Registry 索引探测

- **业务规则**: `probe_registry_index(url)` 尝试访问 registry 的 `index.json`，返回 `(templates, is_not_found)`
- **触发条件**: 用户指定 `--registry` 时判断是市场还是直接下载

### 直接下载

- **业务规则**: `download_registry_direct(cwd, registry, strategy, dest_dir)` 当 registry 没有 `index.json` 时直接 git clone 整个 subdir
- **触发条件**: Registry 探测返回 not_found 时

### 安装路径映射

- **业务规则**: 按模板类型决定安装目录
  - `spec` -> `.harness-cli/spec`
  - `skill` -> `.agents/skills`
  - `command` -> `.claude/commands`
  - `full` -> `.`（仓库根）

### HTTP 代理检测

- **业务规则**: `setup_proxy()` 按优先级读取环境变量 `HTTPS_PROXY` > `https_proxy` > `HTTP_PROXY` > `http_proxy` > `ALL_PROXY` > `all_proxy`
- **触发条件**: init 命令启动时
- **处理流程**: 返回第一个非空值；注意 reqwest 本身会自动读取代理，此函数主要用于日志显示

### 凭证脱敏

- **业务规则**: `mask_proxy_url(url)` 将 URL 中的 `user:pass@` 替换为 `***:***@`
- **触发条件**: 日志中显示代理 URL 时
- **处理流程**: 解析 `scheme://`，找到 `@` 位置替换 user info；无法识别的 URL 整体替换为 `***`

### 模板哈希追踪

- **业务规则**:
  - `compute_hash(content)` 计算 SHA256 并返回十六进制字符串
  - `load_hashes(cwd)` / `save_hashes(cwd, &map)` 读写 `.harness-cli/.template-hashes.json`
  - `initialize_hashes(cwd, &managed_dirs)` 初始化所有受管目录下的文件哈希，返回文件数
  - `update_hashes` / `update_hash_from_file` / `remove_hash` / `rename_hash` 更新哈希条目
  - `is_template_modified(cwd, path, hashes)` 保守判断文件是否被用户修改：文件不存在 -> false；无哈希记录 -> true（保守）；哈希不同 -> true
- **排除规则**: `EXCLUDE_FROM_HASH` 跳过包含以下子串的路径：`.template-hashes.json`、`.version`、`.gitignore`、`.developer`、`workspace/`、`tasks/`、`.current-task`、`spec/`、`.backup-`

## 数据流

```
init --template electron-fullstack
  -> fetch_template_index(默认 URL)
  -> 查找匹配的 SpecTemplate
  -> download_template_by_id
  -> git clone 到临时目录
  -> 按 strategy 合并到 .harness-cli/spec/

init --registry gh:myorg/myrepo/specs
  -> parse_registry_source
  -> probe_registry_index (HTTP GET index.json)
  -> 有索引 -> 按市场流程下载
  -> 无索引 -> download_registry_direct (git clone)

init 完成
  -> all_managed_dirs() -> initialize_hashes
  -> 写入 .harness-cli/.template-hashes.json

update 检查修改
  -> load_hashes -> is_template_modified(path, hashes)
  -> 返回是否被用户修改
```

## 业务规则

- 默认模板市场 URL：`https://raw.githubusercontent.com/mindfold-ai/harness-cli/main/marketplace/index.json`
- 默认模板仓库：`gh:mindfold-ai/harness-cli`
- 模板索引获取超时：5 秒
- 模板下载超时：30 秒
- 代理 URL 脱敏：`user:pass` -> `***:***`；无法识别的 URL 整体替换为 `***`
- 代理环境变量优先级：`HTTPS_PROXY` > `https_proxy` > `HTTP_PROXY` > `http_proxy` > `ALL_PROXY` > `all_proxy`
- 哈希文件存储在 `.harness-cli/.template-hashes.json`（JSON Pretty）
- `.version`、`workspace/`、`tasks/`、`spec/`、`.developer`、`.current-task`、备份文件从哈希追踪中排除
- 无哈希记录的文件一律视为"已修改"（保守策略，保护用户内容）

## 与其他模块的关系

| 模块 | 关系 |
|------|------|
| cli-commands | init 命令的 `--template`/`--registry` 选项触发下载；init 完成后调用 `initialize_hashes` |
| platform-configurators | `all_managed_dirs()` 为 `initialize_hashes` 提供目录列表 |
| version-management | update 命令使用哈希比较检测文件变更 |
| migration-system | SafeFileDelete 通过 `compute_hash` 比对内容 |
| file-management | 哈希追踪只涉及读操作，写入通过 `std::fs::write` 直接处理 |
