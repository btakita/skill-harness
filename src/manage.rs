//! Skill management — install/check/uninstall SKILL.md files for agent environments.
//!
//! CLI tools bundle a SKILL.md via `include_str!` and use this module to install
//! it to the appropriate location for the active agent environment.

use anyhow::{Context, Result};
use std::collections::BTreeSet;
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
        crate::okf::validate_skill_directory_okf(source_dir)?;

        let target_skill = self.skill_path(root);
        let target_dir = target_skill
            .parent()
            .context("target skill path has no parent directory")?;

        sync_directory(source_dir, target_dir)?;
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

    /// Check if the installed skill directory matches the source directory.
    pub fn check_directory(&self, source_dir: &Path, root: Option<&Path>) -> Result<bool> {
        let source_skill = source_dir.join("SKILL.md");
        if !source_skill.is_file() {
            anyhow::bail!(
                "source skill directory must contain SKILL.md: {}",
                source_dir.display()
            );
        }
        crate::okf::validate_skill_directory_okf(source_dir)?;

        let target_skill = self.skill_path(root);
        let target_dir = target_skill
            .parent()
            .context("target skill path has no parent directory")?;

        if !target_dir.exists() {
            eprintln!("Not installed. Run `{} install-dir` to install.", self.name);
            return Ok(false);
        }

        let report = compare_directories(source_dir, target_dir)?;
        if report.is_empty() {
            eprintln!("Directory up to date (v{}).", self.version);
            Ok(true)
        } else {
            for message in report.messages() {
                eprintln!("{message}");
            }
            eprintln!(
                "Outdated. Run `{} install-dir {}` to sync to v{}.",
                self.name, self.name, self.version
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

fn sync_directory(source_dir: &Path, target_dir: &Path) -> Result<()> {
    copy_directory(source_dir, target_dir)?;
    remove_files_not_in_source(source_dir, target_dir)?;
    remove_empty_dirs_not_in_source(source_dir, target_dir)?;
    Ok(())
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

fn remove_files_not_in_source(source_dir: &Path, target_dir: &Path) -> Result<()> {
    let source_files = collect_relative_files(source_dir)?;
    let target_files = collect_relative_files(target_dir)?;
    for rel in target_files {
        if !source_files.contains(&rel) {
            let path = target_dir.join(rel);
            std::fs::remove_file(&path)
                .with_context(|| format!("failed to remove {}", path.display()))?;
        }
    }
    Ok(())
}

fn remove_empty_dirs_not_in_source(source_dir: &Path, target_dir: &Path) -> Result<()> {
    let source_dirs = collect_relative_dirs(source_dir)?;
    let mut target_dirs: Vec<PathBuf> = collect_relative_dirs(target_dir)?.into_iter().collect();
    target_dirs.sort_by_key(|path| std::cmp::Reverse(path.components().count()));

    for rel in target_dirs {
        if source_dirs.contains(&rel) {
            continue;
        }

        let path = target_dir.join(rel);
        if path
            .read_dir()
            .is_ok_and(|mut entries| entries.next().is_none())
        {
            std::fs::remove_dir(&path)
                .with_context(|| format!("failed to remove {}", path.display()))?;
        }
    }

    Ok(())
}

#[derive(Debug, Default, PartialEq, Eq)]
struct DirectoryDiff {
    missing: BTreeSet<PathBuf>,
    changed: BTreeSet<PathBuf>,
    extra: BTreeSet<PathBuf>,
}

impl DirectoryDiff {
    fn is_empty(&self) -> bool {
        self.missing.is_empty() && self.changed.is_empty() && self.extra.is_empty()
    }

    fn messages(&self) -> Vec<String> {
        let mut messages = Vec::new();
        messages.extend(
            self.missing
                .iter()
                .map(|path| format!("Missing installed file: {}", path.display())),
        );
        messages.extend(
            self.changed
                .iter()
                .map(|path| format!("Outdated installed file: {}", path.display())),
        );
        messages.extend(
            self.extra
                .iter()
                .map(|path| format!("Extra installed file: {}", path.display())),
        );
        messages
    }
}

fn compare_directories(source_dir: &Path, target_dir: &Path) -> Result<DirectoryDiff> {
    let source_files = collect_relative_files(source_dir)?;
    let target_files = collect_relative_files(target_dir)?;
    let mut diff = DirectoryDiff::default();

    for rel in &source_files {
        let source_path = source_dir.join(rel);
        let target_path = target_dir.join(rel);
        if !target_path.exists() {
            diff.missing.insert(rel.clone());
            continue;
        }

        let source_bytes = std::fs::read(&source_path)
            .with_context(|| format!("failed to read {}", source_path.display()))?;
        let target_bytes = std::fs::read(&target_path)
            .with_context(|| format!("failed to read {}", target_path.display()))?;
        if source_bytes != target_bytes {
            diff.changed.insert(rel.clone());
        }
    }

    for rel in &target_files {
        if !source_files.contains(rel) {
            diff.extra.insert(rel.clone());
        }
    }

    Ok(diff)
}

fn collect_relative_files(root: &Path) -> Result<BTreeSet<PathBuf>> {
    let mut files = BTreeSet::new();
    collect_relative_files_inner(root, root, &mut files)?;
    Ok(files)
}

fn collect_relative_dirs(root: &Path) -> Result<BTreeSet<PathBuf>> {
    let mut dirs = BTreeSet::new();
    collect_relative_dirs_inner(root, root, &mut dirs)?;
    Ok(dirs)
}

fn collect_relative_dirs_inner(
    root: &Path,
    current: &Path,
    dirs: &mut BTreeSet<PathBuf>,
) -> Result<()> {
    for entry in std::fs::read_dir(current)
        .with_context(|| format!("failed to read {}", current.display()))?
    {
        let entry = entry?;
        if !entry.file_type()?.is_dir() {
            continue;
        }

        let path = entry.path();
        dirs.insert(
            path.strip_prefix(root)
                .with_context(|| {
                    format!("failed to strip {} from {}", root.display(), path.display())
                })?
                .to_path_buf(),
        );
        collect_relative_dirs_inner(root, &path, dirs)?;
    }
    Ok(())
}

fn collect_relative_files_inner(
    root: &Path,
    current: &Path,
    files: &mut BTreeSet<PathBuf>,
) -> Result<()> {
    for entry in std::fs::read_dir(current)
        .with_context(|| format!("failed to read {}", current.display()))?
    {
        let entry = entry?;
        let path = entry.path();
        let file_type = entry.file_type()?;
        if file_type.is_dir() {
            collect_relative_files_inner(root, &path, files)?;
        } else if file_type.is_file() {
            files.insert(
                path.strip_prefix(root)
                    .with_context(|| {
                        format!("failed to strip {} from {}", root.display(), path.display())
                    })?
                    .to_path_buf(),
            );
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
    fn opencode_skill_path() {
        let config = SkillConfig::for_harness(
            "compose-skills",
            "content",
            "1.0.0",
            HarnessTarget::OpenCode,
        );
        assert_eq!(
            config.skill_path(None),
            PathBuf::from(".opencode/skills/compose-skills/SKILL.md")
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
    fn install_directory_copies_opencode_skill_resources() {
        let source = tempfile::tempdir().unwrap();
        std::fs::write(source.path().join("SKILL.md"), "# Compose Skills\n").unwrap();
        std::fs::create_dir_all(source.path().join("references")).unwrap();
        std::fs::write(source.path().join("references/example.md"), "example").unwrap();

        let project = tempfile::tempdir().unwrap();
        let config = SkillConfig::for_harness(
            "compose-skills",
            "# Compose Skills\n",
            "1.0.0",
            HarnessTarget::OpenCode,
        );
        config
            .install_directory(source.path(), Some(project.path()))
            .unwrap();

        assert!(
            project
                .path()
                .join(".opencode/skills/compose-skills/SKILL.md")
                .is_file()
        );
        assert!(
            project
                .path()
                .join(".opencode/skills/compose-skills/references/example.md")
                .is_file()
        );
    }

    #[test]
    fn install_directory_copies_generic_skill_resources() {
        let source = tempfile::tempdir().unwrap();
        std::fs::write(source.path().join("SKILL.md"), "# Compose Skills\n").unwrap();
        std::fs::create_dir_all(source.path().join("references")).unwrap();
        std::fs::write(source.path().join("references/example.md"), "example").unwrap();

        let project = tempfile::tempdir().unwrap();
        let config = SkillConfig::for_harness(
            "compose-skills",
            "# Compose Skills\n",
            "1.0.0",
            HarnessTarget::Generic,
        );
        config
            .install_directory(source.path(), Some(project.path()))
            .unwrap();

        assert!(
            project
                .path()
                .join(".agent/skills/compose-skills/SKILL.md")
                .is_file()
        );
        assert!(
            project
                .path()
                .join(".agent/skills/compose-skills/references/example.md")
                .is_file()
        );
    }

    #[test]
    fn bundled_compose_skills_install_check_matrix_covers_supported_harnesses() {
        let source = tempfile::tempdir().unwrap();
        std::fs::write(source.path().join("SKILL.md"), "# Compose Skills\n").unwrap();
        std::fs::write(source.path().join("SPEC.md"), "# Compose Skills Spec\n").unwrap();
        std::fs::create_dir_all(source.path().join("references/fixtures")).unwrap();
        std::fs::write(
            source.path().join("references/fixtures/example.md"),
            "fixture",
        )
        .unwrap();

        let cases = [
            (
                HarnessTarget::ClaudeCode,
                PathBuf::from(".claude/skills/compose-skills"),
            ),
            (
                HarnessTarget::Codex,
                PathBuf::from(".codex/skills/compose-skills"),
            ),
            (
                HarnessTarget::OpenCode,
                PathBuf::from(".opencode/skills/compose-skills"),
            ),
            (
                HarnessTarget::Generic,
                PathBuf::from(".agent/skills/compose-skills"),
            ),
        ];

        for (target, expected_rel_dir) in cases {
            let project = tempfile::tempdir().unwrap();
            let config =
                SkillConfig::for_harness("compose-skills", "# Compose Skills\n", "1.0.0", target);

            config
                .install_directory(source.path(), Some(project.path()))
                .unwrap();

            let target_dir = project.path().join(expected_rel_dir);
            assert!(target_dir.join("SKILL.md").is_file());
            assert!(target_dir.join("SPEC.md").is_file());
            assert!(target_dir.join("references/fixtures/example.md").is_file());
            assert!(
                config
                    .check_directory(source.path(), Some(project.path()))
                    .unwrap(),
                "installed directory should check clean for {target:?}"
            );
        }
    }

    #[test]
    fn check_directory_accepts_identical_tree() {
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
            config
                .check_directory(source.path(), Some(project.path()))
                .unwrap()
        );
    }

    #[test]
    fn check_directory_reports_missing_changed_and_extra_files() {
        let source = tempfile::tempdir().unwrap();
        std::fs::write(source.path().join("SKILL.md"), "# Compose Skills\n").unwrap();
        std::fs::create_dir_all(source.path().join("references")).unwrap();
        std::fs::write(source.path().join("references/example.md"), "example").unwrap();

        let target = tempfile::tempdir().unwrap();
        std::fs::write(target.path().join("SKILL.md"), "# Old\n").unwrap();
        std::fs::write(target.path().join("extra.md"), "extra").unwrap();

        let diff = compare_directories(source.path(), target.path()).unwrap();
        assert_eq!(diff.changed, BTreeSet::from([PathBuf::from("SKILL.md")]));
        assert_eq!(
            diff.missing,
            BTreeSet::from([PathBuf::from("references/example.md")])
        );
        assert_eq!(diff.extra, BTreeSet::from([PathBuf::from("extra.md")]));
    }

    #[test]
    fn install_directory_removes_stale_installed_files() {
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

        let target_dir = project.path().join(".codex/skills/compose-skills");
        std::fs::create_dir_all(target_dir.join("stale-dir")).unwrap();
        std::fs::write(target_dir.join("stale-dir/old.md"), "old").unwrap();
        std::fs::write(target_dir.join("old.md"), "old").unwrap();

        config
            .install_directory(source.path(), Some(project.path()))
            .unwrap();

        assert!(!target_dir.join("old.md").exists());
        assert!(!target_dir.join("stale-dir").exists());
        assert!(
            config
                .check_directory(source.path(), Some(project.path()))
                .unwrap()
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
