//! Harness CLI -- AI-assisted development workflow framework.

use std::cmp::Ordering;

use clap::{Parser, Subcommand};
use colored::Colorize;

use harness_cli::commands;
use harness_cli::constants::paths::dir_names;
use harness_cli::constants::version::{PACKAGE_NAME, VERSION};
use harness_cli::utils::compare_versions::compare_versions;

// =============================================================================
// CLI definitions
// =============================================================================

#[derive(Parser)]
#[command(
    name = "harness-cli",
    about = "AI-assisted development workflow framework",
    version = VERSION,
)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Initialize harness-cli in the current project
    Init {
        #[arg(long, help = "Include Cursor commands")]
        cursor: bool,
        #[arg(long, help = "Include Claude Code commands")]
        claude: bool,
        #[arg(long, help = "Include iFlow CLI commands")]
        iflow: bool,
        #[arg(long, help = "Include OpenCode commands")]
        opencode: bool,
        #[arg(long, help = "Include Codex skills")]
        codex: bool,
        #[arg(long, help = "Include Kilo CLI commands")]
        kilo: bool,
        #[arg(long, help = "Include Kiro Code skills")]
        kiro: bool,
        #[arg(long, help = "Include Gemini CLI commands")]
        gemini: bool,
        #[arg(long, help = "Include Antigravity workflows")]
        antigravity: bool,
        #[arg(long, help = "Include Windsurf workflows")]
        windsurf: bool,
        #[arg(long, help = "Include Qoder commands")]
        qoder: bool,
        #[arg(long, help = "Include CodeBuddy commands")]
        codebuddy: bool,
        #[arg(long, help = "Include GitHub Copilot hooks")]
        copilot: bool,
        #[arg(short = 'y', long, help = "Skip prompts and use defaults")]
        yes: bool,
        #[arg(
            short = 'u',
            long,
            help = "Initialize developer identity with specified name"
        )]
        user: Option<String>,
        #[arg(short = 'f', long, help = "Overwrite existing files without asking")]
        force: bool,
        #[arg(short = 's', long, help = "Skip existing files without asking")]
        skip_existing: bool,
        #[arg(long, conflicts_with = "no_monorepo", help = "Force monorepo mode")]
        monorepo: bool,
        #[arg(
            long = "no-monorepo",
            conflicts_with = "monorepo",
            help = "Skip monorepo detection"
        )]
        no_monorepo: bool,
        #[arg(
            short = 't',
            long,
            help = "Use a remote spec template (e.g., electron-fullstack)"
        )]
        template: Option<String>,
        #[arg(long, help = "Overwrite existing spec directory when using template")]
        overwrite: bool,
        #[arg(long, help = "Only add missing files when using template")]
        append: bool,
        #[arg(
            short = 'r',
            long,
            help = "Use a custom template registry (e.g., gh:myorg/myrepo/specs)"
        )]
        registry: Option<String>,
    },

    /// Create KB (knowledge base) directory structure with templates
    Scan {
        #[arg(short = 'f', long, help = "Overwrite existing files without asking")]
        force: bool,
    },

    /// Check environment and project health
    Doctor {},

    /// Show project status at a glance
    Status {},

    /// Update harness-cli configuration and commands to latest version
    Update {
        #[arg(long, help = "Preview changes without applying them")]
        dry_run: bool,
        #[arg(short = 'f', long, help = "Overwrite all changed files without asking")]
        force: bool,
        #[arg(short = 's', long, help = "Skip all changed files without asking")]
        skip_all: bool,
        #[arg(short = 'n', long, help = "Create .new copies for all changed files")]
        create_new: bool,
        #[arg(long, help = "Allow downgrading to an older version")]
        allow_downgrade: bool,
        #[arg(long, help = "Apply pending file migrations (renames/deletions)")]
        migrate: bool,
    },
}

// =============================================================================
// Update check
// =============================================================================

fn check_for_updates() {
    let cwd = match std::env::current_dir() {
        Ok(d) => d,
        Err(_) => return,
    };

    let version_file = cwd.join(dir_names::WORKFLOW).join(".version");
    if !version_file.exists() {
        return;
    }

    let project_version = match std::fs::read_to_string(&version_file) {
        Ok(v) => v.trim().to_string(),
        Err(_) => return,
    };

    let cli_version = VERSION;
    let comparison = compare_versions(cli_version, &project_version);

    match comparison {
        Ordering::Greater => {
            println!(
                "{}",
                format!(
                    "\n  Harness CLI update available: {} -> {}",
                    project_version, cli_version
                )
                .yellow()
            );
            println!("{}", "   Run: harness-cli update\n".dimmed());
        }
        Ordering::Less => {
            println!(
                "{}",
                format!(
                    "\n  Your CLI ({}) is older than project ({})",
                    cli_version, project_version
                )
                .yellow()
            );
            println!(
                "{}",
                format!("   Run: npm install -g {}\n", PACKAGE_NAME).dimmed()
            );
        }
        Ordering::Equal => {}
    }
}

// =============================================================================
// Main
// =============================================================================

fn main() {
    // Check for updates at startup (only if .harness-cli exists)
    let cwd = std::env::current_dir().unwrap_or_default();
    if cwd.join(dir_names::WORKFLOW).exists() {
        check_for_updates();
    }

    let cli = Cli::parse();

    let result = match cli.command {
        Commands::Init {
            cursor,
            claude,
            iflow,
            opencode,
            codex,
            kilo,
            kiro,
            gemini,
            antigravity,
            windsurf,
            qoder,
            codebuddy,
            copilot,
            yes,
            user,
            force,
            skip_existing,
            monorepo,
            no_monorepo,
            template,
            overwrite,
            append,
            registry,
        } => {
            let monorepo_opt = if monorepo {
                Some(true)
            } else if no_monorepo {
                Some(false)
            } else {
                None
            };

            commands::init::init(commands::init::InitOptions {
                cursor,
                claude,
                iflow,
                opencode,
                codex,
                kilo,
                kiro,
                gemini,
                antigravity,
                windsurf,
                qoder,
                codebuddy,
                copilot,
                yes,
                user,
                force,
                skip_existing,
                template,
                overwrite,
                append,
                registry,
                monorepo: monorepo_opt,
            })
        }
        Commands::Scan { force } => commands::scan::scan(commands::scan::ScanOptions { force }),
        Commands::Doctor {} => {
            commands::doctor::doctor(commands::doctor::DoctorOptions {})
        }
        Commands::Status {} => {
            commands::status::status(commands::status::StatusOptions {})
        }
        Commands::Update {
            dry_run,
            force,
            skip_all,
            create_new,
            allow_downgrade,
            migrate,
        } => commands::update::update(commands::update::UpdateOptions {
            dry_run,
            force,
            skip_all,
            create_new,
            allow_downgrade,
            migrate,
        }),
    };

    if let Err(e) = result {
        eprintln!("{}", format!("Error: {}", e).red());
        std::process::exit(1);
    }
}
