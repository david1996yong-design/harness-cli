# Harness CLI

> AI capabilities grow like ivy — Harness CLI provides the structure to guide them along a disciplined path.

Harness CLI is a development workflow framework for AI-assisted coding. It sets up structured workflows — task management, session recording, knowledge bases, and multi-agent pipelines — so that AI coding tools work within a disciplined, repeatable process rather than ad-hoc conversations.

## Supported AI Platforms

Claude Code | Cursor | GitHub Copilot | Gemini CLI | Codex | iFlow | OpenCode | Windsurf | Kilo | Kiro | Qoder | CodeBuddy | Antigravity

## Install

### One-line install (Linux / macOS)

```bash
curl -fsSL https://raw.githubusercontent.com/david1996yong-design/harness-cli/master/install.sh | bash
```

### From source (requires Rust toolchain)

```bash
cargo install --git https://github.com/david1996yong-design/harness-cli.git
```

### Manual download

Download the binary for your platform from [GitHub Releases](https://github.com/david1996yong-design/harness-cli/releases), extract it, and place it in your `PATH`.

| Platform | File |
|----------|------|
| Linux x86_64 | `harness-cli-x86_64-unknown-linux-gnu.tar.gz` |
| Linux ARM64 | `harness-cli-aarch64-unknown-linux-gnu.tar.gz` |
| macOS Intel | `harness-cli-x86_64-apple-darwin.tar.gz` |
| macOS Apple Silicon | `harness-cli-aarch64-apple-darwin.tar.gz` |
| Windows x86_64 | `harness-cli-x86_64-pc-windows-msvc.zip` |

## Quick Start

```bash
# Initialize in your project
cd your-project
harness-cli init

# Follow the interactive prompts to:
# 1. Set your developer identity
# 2. Select AI platforms (Claude Code, Cursor, etc.)
# 3. Choose project type (Frontend / Backend / Fullstack)
```

After initialization, your project will have:

```
.harness-cli/
├── config.yaml          # Project configuration
├── workflow.md          # Development workflow guide
├── scripts/             # Python runtime (no external deps)
├── spec/                # Development guidelines
├── workspace/           # Session journals
└── tasks/               # Task management
```

## Commands

### `harness-cli init`

Initialize the workflow in your project. Creates `.harness-cli/` directory, configures selected AI platforms, sets up spec templates, and creates a bootstrap task.

```bash
harness-cli init                          # Interactive setup
harness-cli init --claude --cursor -y     # Non-interactive with specific platforms
harness-cli init -t electron-fullstack    # Use a remote spec template
harness-cli init --monorepo               # Force monorepo mode
```

### `harness-cli scan`

Create knowledge base directory structure with templates (`kb/prd/` and `kb/tech/`).

```bash
harness-cli scan
harness-cli scan --force    # Overwrite existing KB files
```

### `harness-cli update`

Update configuration and commands to the latest CLI version. Detects user-modified files and offers conflict resolution.

```bash
harness-cli update              # Interactive update
harness-cli update --dry-run    # Preview changes without applying
harness-cli update --force      # Overwrite all changed files
harness-cli update --migrate    # Apply pending file migrations
```

## How It Works

1. **Init** scaffolds the workflow directory and AI platform configs
2. **Developers** create tasks, write code with AI assistance, and record sessions in journals
3. **Multi-agent pipelines** can plan, dispatch to isolated worktrees, and merge results
4. **Update** keeps the workflow in sync as the CLI evolves — user modifications are preserved via hash tracking

## Requirements

- Git (initialized repository)
- Python 3.6+ (for runtime scripts; standard library only, no pip install needed)

## License

[AGPL-3.0](LICENSE)
