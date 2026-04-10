# 项目检测

> 自动识别项目类型（Frontend/Backend/Fullstack）和 monorepo 结构

## 模块概述

项目检测模块通过扫描项目根目录中的特征文件和依赖列表来判断项目类型和 monorepo 结构。init 命令使用检测结果决定创建哪些 spec 模板（backend/frontend/both）以及是否为 monorepo 中的每个包创建独立的 spec 目录。支持多种 workspace 管理器：pnpm、npm/yarn、Cargo、Go workspaces、uv、lerna，以及通过 `.gitmodules` 识别的 git 子模块。

## 关键文件

| 文件路径 | 职责 |
|----------|------|
| `src/utils/project_detector.rs` | 项目类型检测、monorepo 检测、workspace glob 展开、包名读取、子模块识别 |

## 核心功能

### 项目类型检测

- **业务规则**: 根据特征文件和 `package.json` 依赖判断项目类型
- **触发条件**: init 命令执行时
- **处理流程**:
  1. 遍历 `FRONTEND_INDICATORS`（package.json、vite/next/nuxt/webpack/svelte/astro/angular/vue 配置、App.tsx/jsx/vue、pages/app 路由入口等），支持 `*` 通配符
  2. 遍历 `BACKEND_INDICATORS`（go.mod/go.sum、Cargo.toml、requirements.txt、pyproject.toml、pom.xml、build.gradle、Gemfile、composer.json、*.csproj、mix.exs、server.ts 等）
  3. 读取 `package.json` 的 `dependencies` 和 `devDependencies`，检查是否包含 `FRONTEND_DEPS`（react、vue、svelte、angular、next、nuxt、astro、solid-js、preact、lit、@remix-run/react）或 `BACKEND_DEPS`（express、fastify、hono、koa、hapi、nest、@nestjs/core、fastapi、flask、django）
  4. 组合判断：两者都有 -> `Fullstack`，只有前端 -> `Frontend`，只有后端 -> `Backend`，都没有 -> `Unknown`

### 项目类型描述

- **业务规则**: `get_project_type_description(type)` 返回人类可读的类型描述
- **触发条件**: CLI 打印检测结果时

### Monorepo 检测

- **业务规则**: `detect_monorepo(cwd)` 按优先级尝试所有支持的 workspace 格式，合并结果并标记 git submodule
- **处理流程**:
  1. 优先解析 `.gitmodules` 构建子模块路径集合
  2. 依次尝试：`pnpm-workspace.yaml`、`package.json` 的 `workspaces`（数组或 yarn v1 对象形式）、`Cargo.toml` 的 `[workspace]`、`go.work` 的 `use` 指令、`pyproject.toml` 的 `[tool.uv.workspace]`
  3. 展开 workspace glob 模式（`packages/*`、支持 `!` 排除）
  4. 为每个包路径读取包名（`package.json` name、`Cargo.toml` `[package] name`、`go.mod` module、`pyproject.toml` `[project] name`，回退到目录名）
  5. 对每个包调用 `detect_project_type` 识别类型
  6. 标记包是否为 git submodule
  7. 去重（同一路径只保留一个）并返回 `Vec<DetectedPackage>`；无 monorepo 返回 `None`

### Workspace glob 展开

- **业务规则**: `expand_workspace_globs` 支持 `*` 通配符（单层目录），`!` 前缀表示排除
- **处理流程**: 递归匹配路径段，对 `*` 段读取目录并过滤点开头目录

### 包名读取

- **业务规则**: `read_package_name(cwd, pkg_path)` 优先级读取：`package.json` name -> `Cargo.toml [package] name` -> `go.mod` module 最后一段 -> `pyproject.toml [project] name` -> 回退为目录名

### 包名清理

- **业务规则**: `sanitize_pkg_name(name)` 去除 `@scope/` 前缀用于目录名
- **触发条件**: 为 monorepo 包创建 `spec/<name>/` 目录时
- **处理流程**: 正则 `^@[^/]+/` 替换为空；返回如 `@zhubao/desktop` -> `desktop`

## 数据流

```
项目根目录
  -> 扫描 FRONTEND_INDICATORS / BACKEND_INDICATORS
  -> 可选：读取 package.json 依赖
  -> 判断 ProjectType (Frontend/Backend/Fullstack/Unknown)

并行：
  -> 解析 .gitmodules -> submodule 集合
  -> 尝试 pnpm/npm/Cargo/Go/uv workspace
  -> 展开 glob -> 读取每个包的 name 和 type
  -> 返回 Vec<DetectedPackage>
```

## 业务规则

- Frontend 特征文件包括：package.json、各种前端框架配置文件、App.tsx/jsx/vue、pages/app 路由
- Backend 特征文件包括：Cargo.toml、go.mod、requirements.txt、pyproject.toml、pom.xml、*.csproj、mix.exs 等
- 前后端特征文件或依赖都存在时判定为 `Fullstack`
- `Unknown` 类型在 init 中被视为 `Fullstack`（创建两者的 spec 模板）
- monorepo 检测按优先级合并结果：pnpm > npm/yarn/bun > Cargo > Go > uv > git submodules
- workspace glob `*` 只匹配单层非点开头目录
- workspace glob 的 `!` 前缀表示排除
- 同一路径出现在多个 workspace 管理器中时去重，git submodule 信息会被保留
- 子模块在 `DetectedPackage` 中通过 `is_submodule: true` 标记

## 与其他模块的关系

| 模块 | 关系 |
|------|------|
| cli-commands | init 命令调用 `detect_project_type`、`detect_monorepo`、`sanitize_pkg_name` |
| platform-configurators | `workflow.rs` 使用 `ProjectType` 和 `DetectedPackage` 决定 spec 结构 |
