# 全量扫描：生成系统架构知识库

扫描项目代码库，从架构维度分析系统，生成 `kb/tech/` 下的 5 个固定文档。

**使用时机**: 首次建立系统架构知识库，或需要完整重建时

---

## 前置条件

- `.harness-cli/kb/tech/` 目录已存在
- `_module-template.md` 模板已存在

如果目录不存在，提示用户先运行 `harness-cli scan`。

---

## 执行步骤

### 步骤 1: 确认目录结构

检查 KB tech 目录是否完整：

```bash
ls .harness-cli/kb/tech/
```

确认以下文件存在：
- `.harness-cli/kb/tech/_module-template.md` - 文档模板与写作指引
- `.harness-cli/kb/tech/index.md` - 文档索引

如果以上文件不存在，提示用户先运行 `harness-cli scan` 创建目录结构。

### 步骤 2: 读取模板

```bash
cat .harness-cli/kb/tech/_module-template.md
```

理解每个文档的结构要求，后续生成时需要严格遵循模板格式。

### 步骤 3: 分析项目结构（架构维度）

系统性地分析项目，关注以下架构维度：

#### 3.1 技术栈识别

- 编程语言及版本
- 框架及版本
- 构建工具（Cargo / npm / webpack 等）
- 依赖管理（Cargo.toml / package.json 等）
- 测试框架

```bash
# 读取项目配置文件
cat Cargo.toml 2>/dev/null
cat package.json 2>/dev/null
cat go.mod 2>/dev/null
cat pyproject.toml 2>/dev/null
cat requirements.txt 2>/dev/null
```

#### 3.2 组件边界和依赖关系

- 目录结构 -> 模块划分
- 模块间的 import/use 关系
- 公共接口（pub mod / export）

```bash
# 查看项目目录结构
find src/ -type f -name "*.rs" | head -50  # Rust
find src/ -type f -name "*.ts" -o -name "*.tsx" | head -50  # TypeScript
```

#### 3.3 核心数据结构 / Schema

- struct / interface / type 定义
- 配置文件格式（JSON / YAML / TOML）
- 数据库 Schema（如有）

#### 3.4 跨模块共享基础设施

- 工具函数（utils/）
- 中间件
- 配置加载
- 错误处理模式
- 日志系统

### 步骤 4: 生成 5 个固定文档

根据分析结果，按照模板格式生成以下文档：

#### 4.1 overview.md -- 系统全景

```bash
# 写入 overview.md
cat > .harness-cli/kb/tech/overview.md << 'CONTENT'
（根据步骤 3 的分析结果，按模板格式填写）
CONTENT
```

包含：
- 技术栈清单（语言、框架、构建工具、依赖）
- 核心组件一句话描述
- 系统边界（外部依赖、输入输出接口）

#### 4.2 component-map.md -- 组件关系

包含：
- 依赖关系图（ASCII 或文字描述）
- 调用链表格
- 数据流描述
- 依赖方向原则

#### 4.3 data-models.md -- 核心数据结构

包含：
- 每个核心数据结构的 Schema
- 字段说明
- 使用示例

#### 4.4 decisions.md -- 架构决策记录

包含：
- 已识别的技术选型决策
- 每个决策的背景、选项、决定和后果
- 使用 ADR-lite 格式

#### 4.5 cross-cutting.md -- 横切关注点

包含：
- 错误处理与传播策略
- 日志管道
- 配置管理
- 共享工具函数清单
- 中间件 / 拦截器（如有）

### 步骤 5: 更新 index.md

更新 `.harness-cli/kb/tech/index.md`，在文档索引表中填写每个文档的实际简述：

```bash
cat > .harness-cli/kb/tech/index.md << 'CONTENT'
（根据生成结果更新索引）
CONTENT
```

### 步骤 6: 输出摘要

完成后，输出扫描摘要：

```
## KB Tech 全量扫描完成

### 技术栈
- 语言: ...
- 框架: ...
- 构建工具: ...

### 生成的文档
- overview.md: 系统全景（N 个组件）
- component-map.md: 组件关系（N 条调用链）
- data-models.md: 核心数据结构（N 个 Schema）
- decisions.md: 架构决策（N 条 ADR）
- cross-cutting.md: 横切关注点（N 个维度）

### 建议后续操作
- 人工审核生成的文档，补充 AI 无法推断的决策背景
- 团队成员补充 decisions.md 中的历史决策
```

---

## 与其他命令的关系

```
知识库维护流程:
  scan (CLI)        -> 创建 kb/tech/ 目录结构
  scan-kb-tech (AI) -> 全量生成 5 个固定文档
  （后续）update-kb-tech -> 增量更新
```

- `harness-cli scan` - CLI 命令，创建目录和模板文件
- `scan-kb-tech` - AI 命令，分析项目并生成文档内容（本命令）
- `update-kb` - AI 命令，增量更新 kb/prd（产品知识库）

---

## 核心原则

> **架构知识库关注「系统怎么搭的」，而非「代码怎么写」（spec）或「产品做什么」（kb/prd）。**
> **记录事实和决策，不记录理想。AI 需要理解系统的真实状态。**
> **所有文档使用中文撰写。**
