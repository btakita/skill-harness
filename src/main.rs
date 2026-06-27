use anyhow::Result;
use clap::{Parser, Subcommand};
use std::path::PathBuf;

#[derive(Parser)]
#[command(
    name = "skill-harness",
    about = "Lifecycle management for AI agent skills"
)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Install a skill to the project
    Install {
        /// Skill name
        name: String,
        /// Skill content file path
        #[arg(short, long)]
        file: PathBuf,
        /// Target harness (auto, claude, codex, opencode, cursor, generic)
        #[arg(long, default_value = "auto")]
        harness: String,
        /// Project root (default: CWD)
        #[arg(short, long)]
        root: Option<PathBuf>,
    },
    /// Install a full skill directory to the project
    InstallDir {
        /// Skill name
        name: String,
        /// Source skill directory containing SKILL.md (defaults to bundled compose-skills)
        #[arg(short, long)]
        source: Option<PathBuf>,
        /// Target harness (auto, claude, codex, opencode, cursor, generic)
        #[arg(long, default_value = "auto")]
        harness: String,
        /// Project root (default: CWD)
        #[arg(short, long)]
        root: Option<PathBuf>,
    },
    /// Check if an installed skill directory is up to date
    CheckDir {
        /// Skill name
        name: String,
        /// Source skill directory containing SKILL.md (defaults to bundled compose-skills)
        #[arg(short, long)]
        source: Option<PathBuf>,
        /// Target harness (auto, claude, codex, opencode, cursor, generic)
        #[arg(long, default_value = "auto")]
        harness: String,
        /// Project root (default: CWD)
        #[arg(short, long)]
        root: Option<PathBuf>,
    },
    /// Check if a skill is installed and up to date
    Check {
        /// Skill name
        name: String,
        /// Skill content file path
        #[arg(short, long)]
        file: PathBuf,
        /// Target harness (auto, claude, codex, opencode, cursor, generic)
        #[arg(long, default_value = "auto")]
        harness: String,
        /// Project root (default: CWD)
        #[arg(short, long)]
        root: Option<PathBuf>,
    },
    /// Uninstall a skill from the project
    Uninstall {
        /// Skill name
        name: String,
        /// Target harness (auto, claude, codex, opencode, cursor, generic)
        #[arg(long, default_value = "auto")]
        harness: String,
        /// Project root (default: CWD)
        #[arg(short, long)]
        root: Option<PathBuf>,
    },
    /// Validate skill composition architecture plans
    Compose {
        #[command(subcommand)]
        command: ComposeCommands,
    },
    /// Validate Open Knowledge Format bundles
    Okf {
        #[command(subcommand)]
        command: OkfCommands,
    },
    /// List installed skills
    List {
        /// Project root (default: CWD)
        #[arg(short, long)]
        root: Option<PathBuf>,
    },
}

fn main() -> Result<()> {
    let cli = Cli::parse();

    match cli.command {
        Commands::Install {
            name,
            file,
            harness,
            root,
        } => {
            let content = std::fs::read_to_string(&file)?;
            for message in skill_harness::okf::single_file_resource_warnings(&file, &content) {
                eprintln!("warning: {message}");
            }
            let config = make_config(&name, &content, &harness)?;
            config.install(root.as_deref())?;
        }
        Commands::InstallDir {
            name,
            source,
            harness,
            root,
        } => {
            let source = resolve_skill_source(&name, source)?;
            let content = std::fs::read_to_string(source.join("SKILL.md"))?;
            let config = make_config(&name, &content, &harness)?;
            config.install_directory(&source, root.as_deref())?;
        }
        Commands::CheckDir {
            name,
            source,
            harness,
            root,
        } => {
            let source = resolve_skill_source(&name, source)?;
            let content = std::fs::read_to_string(source.join("SKILL.md"))?;
            let config = make_config(&name, &content, &harness)?;
            let ok = config.check_directory(&source, root.as_deref())?;
            if !ok {
                std::process::exit(1);
            }
        }
        Commands::Check {
            name,
            file,
            harness,
            root,
        } => {
            let content = std::fs::read_to_string(&file)?;
            let config = make_config(&name, &content, &harness)?;
            let ok = config.check(root.as_deref())?;
            if !ok {
                std::process::exit(1);
            }
        }
        Commands::Uninstall {
            name,
            harness,
            root,
        } => {
            let config = make_config(&name, "", &harness)?;
            config.uninstall(root.as_deref())?;
        }
        Commands::Compose { command } => match command {
            ComposeCommands::Validate { plan } => {
                let report = skill_harness::compose::validate_composition_plan_path(&plan)?;
                if report.is_valid() {
                    println!(
                        "ok: {} defines {} candidate skill(s)",
                        plan.display(),
                        report.candidate_names.len()
                    );
                } else {
                    for message in report.error_messages() {
                        eprintln!("{message}");
                    }
                    std::process::exit(1);
                }
            }
        },
        Commands::Okf { command } => match command {
            OkfCommands::Validate { path } => {
                let report = skill_harness::okf::validate_okf_bundle(&path)?;
                println!(
                    "OKF bundle: {} ({} concept file(s), {} warning(s), {} error(s))",
                    path.display(),
                    report.concept_count,
                    report.warnings.len(),
                    report.errors.len()
                );
                for message in report.warning_messages() {
                    eprintln!("warning: {message}");
                }
                if !report.is_valid() {
                    for message in report.error_messages() {
                        eprintln!("{message}");
                    }
                    std::process::exit(1);
                }
            }
        },
        Commands::List { root } => {
            let root = root.unwrap_or_else(|| PathBuf::from("."));
            list_skills(&root);
        }
    }

    Ok(())
}

#[derive(Subcommand)]
enum ComposeCommands {
    /// Validate a compose-skills architecture plan
    Validate {
        /// Markdown plan path
        plan: PathBuf,
    },
}

#[derive(Subcommand)]
enum OkfCommands {
    /// Validate an OKF bundle directory
    Validate {
        /// OKF bundle root containing Markdown concept files
        path: PathBuf,
    },
}

fn make_config(
    name: &str,
    content: &str,
    harness: &str,
) -> Result<skill_harness::manage::SkillConfig> {
    if harness != "auto" {
        let target = skill_harness::manage::HarnessTarget::parse(harness)
            .ok_or_else(|| anyhow::anyhow!("unknown harness target: {harness}"))?;
        return Ok(skill_harness::manage::SkillConfig::for_harness(
            name,
            content,
            env!("CARGO_PKG_VERSION"),
            target,
        ));
    }

    Ok(make_auto_config(name, content))
}

fn make_auto_config(name: &str, content: &str) -> skill_harness::manage::SkillConfig {
    #[cfg(feature = "detect")]
    {
        skill_harness::manage::skill_for_environment(name, content, env!("CARGO_PKG_VERSION"))
    }
    #[cfg(not(feature = "detect"))]
    {
        skill_harness::manage::SkillConfig::generic(name, content, env!("CARGO_PKG_VERSION"))
    }
}

fn resolve_skill_source(name: &str, source: Option<PathBuf>) -> Result<PathBuf> {
    if let Some(source) = source {
        return Ok(source);
    }

    if name == "compose-skills" {
        return Ok(PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("skills/compose-skills"));
    }

    Err(anyhow::anyhow!(
        "--source is required for {name}; only compose-skills has a bundled canonical source"
    ))
}

fn list_skills(root: &std::path::Path) {
    let patterns = [
        ".agent/skills/*/SKILL.md",
        ".claude/skills/*/SKILL.md",
        ".codex/skills/*/SKILL.md",
        ".opencode/skills/*/SKILL.md",
    ];

    let mut found = false;
    for pattern in &patterns {
        let full = root.join(pattern).to_string_lossy().to_string();
        if let Ok(entries) = glob::glob(&full) {
            for entry in entries.flatten() {
                let name = entry
                    .parent()
                    .and_then(|p| p.file_name())
                    .and_then(|n| n.to_str())
                    .unwrap_or("unknown");
                let rel = entry.strip_prefix(root).unwrap_or(&entry);
                println!("  {} → {}", name, rel.display());
                found = true;
            }
        }
    }

    if !found {
        println!("No skills installed.");
    }
}
