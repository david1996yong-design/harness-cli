# 增量更新产品知识库

基于 git diff 的增量更新，只更新受代码变更影响的 KB 模块文档，避免全量扫描开销。

**使用时机**: 代码变更后，需要同步更新 KB 文档时

---

## 前置条件

- KB 目录 `.harness-cli/kb/prd/` 已存在
- 模块模板 `_module-template.md` 已存在
- 索引文件 `index.md` 已存在
- 至少已有一次全量扫描（存在模块文档）

---

## 执行步骤

### 步骤 1: 确认 KB 目录和模块文档

检查 KB 目录结构是否完整：

```bash
ls .harness-cli/kb/prd/
```

确认以下文件存在：
- `.harness-cli/kb/prd/_module-template.md` - 模块文档模板
- `.harness-cli/kb/prd/index.md` - 模块索引

如果以上文件不存在，提示用户先运行全量扫描命令。

### 步骤 2: 读取模块模板

```bash
cat .harness-cli/kb/prd/_module-template.md
```

理解模块文档的标准结构，后续更新时需要遵循此模板格式。

### 步骤 3: 读取当前模块索引

```bash
cat .harness-cli/kb/prd/index.md
```

了解当前所有已注册的模块及其简述。

### 步骤 4: 获取变更文件列表

通过 git diff 获取近期变更的文件列表。

**默认模式**（最近 10 次提交）：

```bash
git diff --name-only HEAD~10
```

**用户指定 commit range**：

如果用户提供了特定的 commit range 或分支对比参数，使用用户指定的范围：

```bash
# 示例：对比特定分支
git diff --name-only main...HEAD

# 示例：指定 commit 范围
git diff --name-only <commit-hash-1>..<commit-hash-2>

# 示例：最近 N 次提交
git diff --name-only HEAD~N
```

> **提示**: 如果用户未指定范围，默认使用 `HEAD~10`。将获取到的文件列表保存为 `CHANGED_FILES`。

过滤掉非源码文件（如 `.md`、`.json` 配置、`.gitignore` 等），专注于实际的代码变更。

### 步骤 5: 建立文件到模块的映射

逐个读取 `.harness-cli/kb/prd/` 下的每个模块文档（排除 `_module-template.md` 和 `index.md`）。

对每个模块文档：
1. 找到「关键文件」表（通常是一个包含文件路径的表格或列表）
2. 提取该模块关联的所有文件路径
3. 建立映射关系：`文件路径 -> 模块名称`

```
映射结果示例：
  src/commands/init.rs -> cli-commands.md
  src/commands/update.rs -> cli-commands.md
  src/template/engine.rs -> template-engine.md
  src/config/loader.rs -> config-system.md
```

### 步骤 6: 过滤受影响的模块

将 `CHANGED_FILES`（步骤 4）与文件-模块映射（步骤 5）进行匹配：

- 对每个变更文件，查找其所属模块
- 汇总所有受影响的模块列表（去重）
- 记录哪些变更文件不属于任何现有模块（`UNMAPPED_FILES`）

```
受影响模块示例：
  cli-commands.md (因为 src/commands/init.rs 有变更)
  template-engine.md (因为 src/template/engine.rs 有变更)

未映射文件：
  src/new-feature/handler.rs (不属于任何现有模块)
```

### 步骤 7: 增量更新受影响的模块

对每个受影响的模块执行以下操作：

1. **读取现有模块文档**：

```bash
cat .harness-cli/kb/prd/<module-name>.md
```

2. **读取该模块相关的变更源文件**：

```bash
cat <changed-source-file>
```

3. **增量更新文档内容**：
   - 保持模块文档的整体结构不变（遵循模板格式）
   - 更新「模块概述」中因变更而过时的描述
   - 更新「关键文件」表（新增/移除文件）
   - 更新「核心功能」章节中受影响的功能描述
   - 更新「数据流」或「接口」等章节（如有变更）
   - 保留未受变更影响的内容不变

> **重要**: 只修改与变更相关的内容，不要重写整个文档。保持增量更新的原则。

### 步骤 8: 检测并创建新模块

对步骤 6 中识别的 `UNMAPPED_FILES`（不属于任何现有模块的变更文件）：

1. 分析这些文件的功能和所属领域
2. 判断是否需要创建新模块文档：
   - 如果文件属于一个全新的功能领域 -> 创建新模块
   - 如果文件应该归属于某个现有模块（只是尚未被记录）-> 更新现有模块的关键文件表
3. 对需要创建的新模块：
   - 使用 `_module-template.md` 作为模板
   - 读取相关源文件，填充模块文档内容
   - 提示用户确认新模块的名称和范围

```
提示示例：
  发现以下变更文件不属于任何现有模块：
  - src/new-feature/handler.rs
  - src/new-feature/types.rs

  建议创建新模块文档: new-feature.md
  是否确认创建？
```

### 步骤 9: 检测已删除的模块

对每个现有模块文档，检查其「关键文件」表中的所有文件是否仍然存在：

```bash
# 对每个模块的关键文件执行检查
test -f <file-path> && echo "exists" || echo "missing"
```

- 如果一个模块的**所有**关键文件都不存在 -> 标记为已删除模块
- 提示用户确认是否从 `index.md` 中移除该模块
- 如果确认，删除模块文档并更新索引

```
提示示例：
  模块 legacy-feature.md 的所有关键文件已不存在：
  - src/legacy/handler.rs (已删除)
  - src/legacy/types.rs (已删除)

  建议从索引中移除此模块。是否确认？
```

### 步骤 10: 更新索引文件

更新 `.harness-cli/kb/prd/index.md`：

1. 更新已修改模块的简述（如果模块概述有变化）
2. 添加新创建的模块条目
3. 移除已删除的模块条目
4. 确保索引中的模块列表与实际模块文档一致

---

## 输出报告

完成后，输出更新摘要：

```
## KB 增量更新完成

### 变更检测范围
- 检测范围: HEAD~10 (或用户指定的范围)
- 变更文件数: N 个

### 更新的模块
- <module-name>.md: 更新了 <具体变更内容>
- ...

### 新增的模块
- <new-module>.md: <模块简述>
- ...

### 删除的模块
- <removed-module>.md: 所有关键文件已不存在
- ...

### 未处理的文件
- <file-path>: 未映射到任何模块（非源码文件/配置文件）
- ...
```

---

## 与其他命令的关系

```
KB 维护流程:
  全量扫描 (scan-kb) -> 日常开发 -> 增量更新 (update-kb) -> 日常开发 -> ...
                                        |
                              只更新受影响的模块，效率更高
```

- `scan-kb` - 全量扫描，生成/重建所有模块文档（首次使用或需要完整重建时）
- `update-kb` - 增量更新，只处理变更相关的模块（日常维护，本命令）

---

## 核心原则

> **知识库应与代码保持同步。增量更新既保证了文档的时效性，又避免了不必要的全量扫描开销。**
