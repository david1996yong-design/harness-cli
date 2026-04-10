# 文件管理

> 提供带冲突处理的文件写入、目录创建和权限管理功能

## 模块概述

文件管理模块封装了所有文件系统操作，核心是 `write_file` 函数的冲突处理机制。当目标文件已存在且内容不同时，根据全局写入模式（Ask/Force/Skip/Append）采取不同策略。这确保了 init 和 update 命令不会意外覆盖用户的自定义文件。写入模式通过 `AtomicU8` 全局共享，允许用户在交互模式下选择"全局"选项批量处理后续冲突。

## 关键文件

| 文件路径 | 职责 |
|----------|------|
| `src/utils/file_writer.rs` | 文件写入核心：`WriteMode` 枚举、`write_file` 冲突处理、`ensure_dir`、`set_write_mode`/`get_write_mode`、`set_executable`（Unix） |
| `src/utils/mod.rs` | 工具模块导出 |

## 核心功能

### 冲突感知的文件写入

- **业务规则**: `write_file(path, content, executable)` 根据全局 `WriteMode` 处理文件冲突，返回 `Ok(bool)` 表示是否实际写入
- **触发条件**: 任何需要写入文件的操作
- **处理流程**:
  1. 文件不存在 -> 直接写入，可选设置可执行权限，返回 `Ok(true)`
  2. 文件存在且内容完全相同 -> 静默跳过，返回 `Ok(false)`（不受 WriteMode 影响）
  3. 文件存在但内容不同 -> 按 WriteMode 处理：
     - **Force**: 直接覆盖，黄色打印 "Overwritten"
     - **Skip**: 跳过，灰色打印 "Skipped"
     - **Append**: 调用 `append_to_file`，蓝色打印 "Appended"
     - **Ask**: 交互式 `Select` 提示 6 个选项：
       - Skip (keep existing)
       - Overwrite
       - Append to end
       - Skip all remaining conflicts -> 同时 `set_write_mode(Skip)`
       - Overwrite all remaining conflicts -> 同时 `set_write_mode(Force)`
       - Append all remaining conflicts -> 同时 `set_write_mode(Append)`

### 追加到文件

- **业务规则**: `append_to_file(path, content, executable)` 将内容追加到现有文件，自动确保换行
- **处理流程**: 读取现有内容 -> 若不以 `\n` 结尾则插入换行 -> 写回 -> 可选设置权限

### 全局写入模式

- **业务规则**: `WriteMode` 是全局原子状态，一旦用户选择"全局"选项，后续所有冲突自动按该模式处理
- **触发条件**: init/update 命令启动时通过 `--force`/`--skip-existing` 预设，或交互模式下用户选择 "all remaining" 选项
- **处理流程**: 通过 `AtomicU8` 存储全局模式，`set_write_mode` / `get_write_mode` 读写；`WriteMode::from_u8` 把数值映射回枚举，未知值回退为 `Ask`

### 目录创建

- **业务规则**: `ensure_dir(path)` 递归创建目录（等价于 `mkdir -p`）
- **触发条件**: 写入文件前确保父目录存在
- **处理流程**: 调用 `std::fs::create_dir_all`，对已存在目录不报错

### 可执行权限

- **业务规则**: Unix 系统上对传入 `executable=true` 的文件设置 0755 权限
- **触发条件**: 复制 `.py`/`.sh` 脚本或调用方显式要求时
- **处理流程**: 调用 `PermissionsExt::set_mode(0o755)`，非 Unix 系统为空操作

### 相对路径显示

- **业务规则**: `get_relative_path(file_path)` 返回相对于 cwd 的显示路径，用于用户可读的日志输出
- **处理流程**: 尝试 `strip_prefix(cwd)`，失败时回退到完整路径

## 数据流

```
调用方传入 (文件路径, 内容, 是否可执行)
  -> 检查文件是否存在
  -> 存在时比较内容字符串
  -> 根据 WriteMode 决定操作
  -> 写入/跳过/追加（可能弹出交互 Select）
  -> 可选设置可执行权限
  -> 返回 Ok(bool) 表示是否实际写入
```

## 业务规则

- 文件内容完全相同时始终静默跳过（不受 WriteMode 影响）
- Append 模式在文件末尾不是换行符时自动添加换行符
- WriteMode 使用 `AtomicU8` 确保线程安全，顺序一致性
- 默认模式为 `Ask`（交互式）
- 用户在 Ask 模式下选择 "all remaining" 选项后，全局 WriteMode 会被修改，影响后续所有 `write_file` 调用
- 日志输出使用 colored crate，自动适配终端颜色

## 与其他模块的关系

| 模块 | 关系 |
|------|------|
| cli-commands | init 的 `--force`/`--skip-existing` flag 设置全局 `WriteMode`；scan 的 `--force` 同理 |
| platform-configurators | 所有配置器通过 `write_file` 写入文件 |
| template-system | `copy_embedded_dir` 内部调用 `write_file` |
