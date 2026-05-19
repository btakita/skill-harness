//! Skill management — install/check/uninstall SKILL.md files for agent environments.
//!
//! CLI tools bundle a SKILL.md via `include_str!` and use this module to install
//! it to the appropriate location for the active agent environment.

use anyhow::{Context, Result};
use std::path::{Path, PathBuf};

/// Explicit harness target for deterministic install/check/uninstall behavior.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HarnessTarget {
    ClaudeCode,
    Codex,
    OpenCode,
    Cursor,
    Generic,
}

impl HarnessTarget {
    pub fn parse(name: &str) -> Option<Self> {
        match name.to_ascii_lowercase().as_str() {
            "claude" | "claude-code" | "claudecode" => Some(Self::ClaudeCode),
            "codex" => Some(Self::Codex),
            "opencode" | "open-code" => Some(Self::OpenCode),
            "cursor" => Some(Self::Cursor),
            "generic" | "agent" => Some(Self::Generic),
            _ => None,
        }
    }

    pub fn skill_rel_path(&self, name: &str) -> PathBuf {
        match self {
            Self::ClaudeCode => PathBuf::from(format!(".claude/skills/{name}/SKILL.md")),
            Self::Codex => PathBuf::from(format!(".codex/skills/{name}/SKILL.md")),
            Self::OpenCode => PathBuf::from(format!(".opencode/skills/{name}/SKILL.md")),
            Self::Cursor => PathBuf::from(format!(".cursor/rules/{name}.md")),
            Self::Generic => PathBuf::from(format!(".agent/skills/{name}/SKILL.md")),
        }
    }
}

/// Configuration for a skill to be managed.
pub struct SkillConfig {
    /// The tool name (e.g., "agent-doc", "webmaster").
    pub name: String,
    /// The bundled SKILL.md content (typically from `include_str!`).
    pub content: String,
    /// The tool version (typically from `env!("CARGO_PKG_VERSION")`).
    pub version: String,
    /// The relative path resolver for the target environment.
    pub path_resolver: Box<dyn Fn(&str) -> PathBuf + Send + Sync>,
}

impl SkillConfig {
    /// Create a new skill config with a custom path resolver.
    pub fn new(
        name: impl Into<String>,
        content: impl Into<String>,
        version: impl Into<String>,
        path_resolver: impl Fn(&str) -> PathBuf + Send + Sync + 'static,
    ) -> Self {
        Self {
            name: name.into(),
            content: content.into(),
            version: version.into(),
            path_resolver: Box::new(path_resolver),
        }
    }

    /// Create a skill config that installs to `.agent/skills/<name>/SKILL.md`.
    pub fn generic(
        name: impl Into<String>,
        content: impl Into<String>,
        version: impl Into<String>,
    ) -> Self {
        Self::for_harness(name, content, version, HarnessTarget::Generic)
    }

    /// Create a skill config for a specific harness target.
    pub fn for_harness(
        name: impl Into<String>,
        content: impl Into<String>,
        version: impl Into<String>,
        target: HarnessTarget,
    ) -> Self {
        Self::new(name, content, version, move |name| {
            target.skill_rel_path(name)
        })
    }

    /// Resolve the skill file path under the given root (or CWD if None).
    pub fn skill_path(&self, root: Option<&Path>) -> PathBuf {
        let rel = (self.path_resolver)(&self.name);
        match root {
            Some(r) => r.join(rel),
            None => rel,
        }
    }

    /// Install the bundled SKILL.md to the project.
    pub fn install(&self, root: Option<&Path>) -> Result<()> {
        let path = self.skill_path(root);

        if path.exists() {
            let existing = std::fs::read_to_string(&path)
                .with_context(|| format!("failed to read {}", path.display()))?;
            if existing == self.content {
                eprintln!("Skill already up to date (v{}).", self.version);
                return Ok(());
            }
        }

        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)
                .with_context(|| format!("failed to create {}", parent.display()))?;
        }

        std::fs::write(&path, &self.content)
            .with_context(|| format!("failed to write {}", path.display()))?;
        eprintln!("Installed skill v{} → {}", self.version, path.display());

        Ok(())
    }

    /// Install every file from a portable skill directory into the target skill directory.
    pub fn install_directory(&self, source_dir: &Path, root: Option<&Path>) -> Result<()> {
        let source_skill = source_dir.join("SKILL.md");
        if !source_skill.is_file() {
            anyhow::bail!(
                "source skill directory must contain SKILL.md: {}",
                source_dir.display()
            );
        }

        let target_skill = self.skill_path(root);
        let target_dir = target_skill
            .parent()
            .context("target skill path has no parent directory")?;

        copy_directory(source_dir, target_dir)?;
        eprintln!(
            "Installed skill directory v{} → {}",
            self.version,
            target_dir.display()
        );
        Ok(())
    }

    /// Check if the installed skill matches the bundled version.
    pub fn check(&self, root: Option<&Path>) -> Result<bool> {
        let path = self.skill_path(root);

        if !path.exists() {
            eprintln!(
                "Not installed. Run `{} skill install` to install.",
                self.name
            );
            return Ok(false);
        }

        let existing = std::fs::read_to_string(&path)
            .with_context(|| format!("failed to read {}", path.display()))?;

        if existing == self.content {
            eprintln!("Up to date (v{}).", self.version);
            Ok(true)
        } else {
            eprintln!(
                "Outdated. Run `{} skill install` to update to v{}.",
                self.name, self.version
            );
            Ok(false)
        }
    }

    /// Uninstall the skill file and its parent directory (if empty).
    pub fn uninstall(&self, root: Option<&Path>) -> Result<()> {
        let path = self.skill_path(root);

        if !path.exists() {
            eprintln!("Skill not installed.");
            return Ok(());
        }

        std::fs::remove_file(&path)
            .with_context(|| format!("failed to remove {}", path.display()))?;

        if let Some(parent) = path.parent()
            && parent.read_dir().is_ok_and(|mut d| d.next().is_none())
        {
            let _ = std::fs::remove_dir(parent);
        }

        eprintln!("Uninstalled skill from {}", path.display());
        Ok(())
    }
}

fn copy_directory(source_dir: &Path, target_dir: &Path) -> Result<()> {
    std::fs::create_dir_all(target_dir)
        .with_context(|| format!("failed to create {}", target_dir.display()))?;

    for entry in std::fs::read_dir(source_dir)
        .with_context(|| format!("failed to read {}", source_dir.display()))?
    {
        let entry = entry?;
        let source_path = entry.path();
        let target_path = target_dir.join(entry.file_name());
        let file_type = entry.file_type()?;

        if file_type.is_dir() {
            copy_directory(&source_path, &target_path)?;
        } else if file_type.is_file() {
            if let Some(parent) = target_path.parent() {
                std::fs::create_dir_all(parent)
                    .with_context(|| format!("failed to create {}", parent.display()))?;
            }
            std::fs::copy(&source_path, &target_path).with_context(|| {
                format!(
                    "failed to copy {} to {}",
                    source_path.display(),
                    target_path.display()
                )
            })?;
        }
    }
    Ok(())
}

/// Create a SkillConfig that uses agent-kit Environment for path resolution.
#[cfg(feature = "detect")]
pub fn skill_for_environment(
    name: impl Into<String>,
    content: impl Into<String>,
    version: impl Into<String>,
) -> SkillConfig {
    let env = agent_kit::detect::Environment::detect();
    let name_str = name.into();
    let name_clone = name_str.clone();
    SkillConfig {
        name: name_str,
        content: content.into(),
        version: version.into(),
        path_resolver: Box::new(move |_| env.skill_rel_path(&name_clone)),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_config() -> SkillConfig {
        SkillConfig::for_harness(
            "test-tool",
            "# Test Skill\n\nSome content.\n",
            "1.0.0",
            HarnessTarget::ClaudeCode,
        )
    }

    #[test]
    fn skill_path_with_root() {
        let config = test_config();
        let path = config.skill_path(Some(Path::new("/project")));
        assert_eq!(
            path,
            PathBuf::from("/project/.claude/skills/test-tool/SKILL.md")
        );
    }

    #[test]
    fn skill_path_without_root() {
        let config = test_config();
        let path = config.skill_path(None);
        assert_eq!(path, PathBuf::from(".claude/skills/test-tool/SKILL.md"));
    }

    #[test]
    fn generic_skill_path() {
        let config = SkillConfig::generic("my-tool", "content", "1.0.0");
        let path = config.skill_path(None);
        assert_eq!(path, PathBuf::from(".agent/skills/my-tool/SKILL.md"));
    }

    #[test]
    fn claude_code_skill_path() {
        let config = SkillConfig::for_harness(
            "compose-skills",
            "content",
            "1.0.0",
            HarnessTarget::ClaudeCode,
        );
        assert_eq!(
            config.skill_path(None),
            PathBuf::from(".claude/skills/compose-skills/SKILL.md")
        );
    }

    #[test]
    fn codex_skill_path() {
        let config =
            SkillConfig::for_harness("compose-skills", "content", "1.0.0", HarnessTarget::Codex);
        assert_eq!(
            config.skill_path(None),
            PathBuf::from(".codex/skills/compose-skills/SKILL.md")
        );
    }

    #[test]
    fn install_creates_file() {
        let dir = tempfile::tempdir().unwrap();
        let config = test_config();
        config.install(Some(dir.path())).unwrap();

        let path = dir.path().join(".claude/skills/test-tool/SKILL.md");
        assert!(path.exists());
        let content = std::fs::read_to_string(&path).unwrap();
        assert_eq!(content, config.content);
    }

    #[test]
    fn install_idempotent() {
        let dir = tempfile::tempdir().unwrap();
        let config = test_config();
        config.install(Some(dir.path())).unwrap();
        config.install(Some(dir.path())).unwrap();

        let path = dir.path().join(".claude/skills/test-tool/SKILL.md");
        let content = std::fs::read_to_string(&path).unwrap();
        assert_eq!(content, config.content);
    }

    #[test]
    fn install_directory_copies_claude_skill_resources() {
        let source = tempfile::tempdir().unwrap();
        std::fs::write(source.path().join("SKILL.md"), "# Compose Skills\n").unwrap();
        std::fs::create_dir_all(source.path().join("references")).unwrap();
        std::fs::write(source.path().join("references/example.md"), "example").unwrap();

        let project = tempfile::tempdir().unwrap();
        let config = SkillConfig::for_harness(
            "compose-skills",
            "# Compose Skills\n",
            "1.0.0",
            HarnessTarget::ClaudeCode,
        );
        config
            .install_directory(source.path(), Some(project.path()))
            .unwrap();

        assert!(
            project
                .path()
                .join(".claude/skills/compose-skills/SKILL.md")
                .is_file()
        );
        assert!(
            project
                .path()
                .join(".claude/skills/compose-skills/references/example.md")
                .is_file()
        );
    }

    #[test]
    fn install_directory_copies_codex_skill_resources() {
        let source = tempfile::tempdir().unwrap();
        std::fs::write(source.path().join("SKILL.md"), "# Compose Skills\n").unwrap();
        std::fs::create_dir_all(source.path().join("references")).unwrap();
        std::fs::write(source.path().join("references/example.md"), "example").unwrap();

        let project = tempfile::tempdir().unwrap();
        let config = SkillConfig::for_harness(
            "compose-skills",
            "# Compose Skills\n",
            "1.0.0",
            HarnessTarget::Codex,
        );
        config
            .install_directory(source.path(), Some(project.path()))
            .unwrap();

        assert!(
            project
                .path()
                .join(".codex/skills/compose-skills/SKILL.md")
                .is_file()
        );
        assert!(
            project
                .path()
                .join(".codex/skills/compose-skills/references/example.md")
                .is_file()
        );
    }

    #[test]
    fn check_not_installed() {
        let dir = tempfile::tempdir().unwrap();
        let config = test_config();
        assert!(!config.check(Some(dir.path())).unwrap());
    }

    #[test]
    fn check_up_to_date() {
        let dir = tempfile::tempdir().unwrap();
        let config = test_config();
        config.install(Some(dir.path())).unwrap();
        assert!(config.check(Some(dir.path())).unwrap());
    }

    #[test]
    fn uninstall_removes_file() {
        let dir = tempfile::tempdir().unwrap();
        let config = test_config();
        config.install(Some(dir.path())).unwrap();
        config.uninstall(Some(dir.path())).unwrap();

        let path = dir.path().join(".claude/skills/test-tool/SKILL.md");
        assert!(!path.exists());
    }

    #[test]
    fn uninstall_not_installed() {
        let dir = tempfile::tempdir().unwrap();
        let config = test_config();
        config.uninstall(Some(dir.path())).unwrap();
    }
}
