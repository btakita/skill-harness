//! OKF (Open Knowledge Format) validation for skill resource bundles.

use anyhow::{Context, Result};
use serde_yaml::{Mapping, Value};
use std::collections::BTreeSet;
use std::path::{Path, PathBuf};

const RESOURCE_ROOTS: &[&str] = &["runbooks", "references", "scripts", "assets", "okf"];

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OkfReport {
    pub root: PathBuf,
    pub okf_version: Option<String>,
    pub index_present: bool,
    pub log_present: bool,
    pub concept_count: usize,
    pub files: Vec<OkfFile>,
    pub errors: Vec<OkfIssue>,
    pub warnings: Vec<OkfIssue>,
}

impl OkfReport {
    pub fn is_valid(&self) -> bool {
        self.errors.is_empty()
    }

    pub fn error_messages(&self) -> Vec<String> {
        self.errors.iter().map(OkfIssue::message).collect()
    }

    pub fn warning_messages(&self) -> Vec<String> {
        self.warnings.iter().map(OkfIssue::message).collect()
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SkillDirectoryReport {
    pub root: PathBuf,
    pub okf: Option<OkfReport>,
    pub resource_refs: Vec<ResourceRef>,
    pub errors: Vec<OkfIssue>,
    pub warnings: Vec<OkfIssue>,
}

impl SkillDirectoryReport {
    pub fn is_valid(&self) -> bool {
        self.errors.is_empty()
    }

    pub fn error_messages(&self) -> Vec<String> {
        self.errors.iter().map(OkfIssue::message).collect()
    }

    pub fn warning_messages(&self) -> Vec<String> {
        self.warnings.iter().map(OkfIssue::message).collect()
    }
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct ResourceRef {
    pub path: PathBuf,
    pub line: usize,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OkfFile {
    pub path: PathBuf,
    pub kind: OkfFileKind,
    pub concept_type: Option<String>,
    pub title: Option<String>,
    pub tags: Vec<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OkfFileKind {
    Index,
    Log,
    Concept,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OkfIssue {
    pub path: Option<PathBuf>,
    pub message: String,
}

impl OkfIssue {
    pub fn message(&self) -> String {
        match &self.path {
            Some(path) => format!("{}: {}", path.display(), self.message),
            None => self.message.clone(),
        }
    }
}

pub fn validate_okf_bundle(root: &Path) -> Result<OkfReport> {
    let mut report = OkfReport {
        root: root.to_path_buf(),
        okf_version: None,
        index_present: false,
        log_present: false,
        concept_count: 0,
        files: Vec::new(),
        errors: Vec::new(),
        warnings: Vec::new(),
    };

    if !root.exists() {
        report.errors.push(OkfIssue {
            path: None,
            message: format!("OKF root does not exist: {}", root.display()),
        });
        return Ok(report);
    }
    if !root.is_dir() {
        report.errors.push(OkfIssue {
            path: None,
            message: format!("OKF root is not a directory: {}", root.display()),
        });
        return Ok(report);
    }

    let files = collect_relative_files(root)?;
    for rel in files {
        if rel.extension().and_then(|ext| ext.to_str()) != Some("md") {
            report.warnings.push(OkfIssue {
                path: Some(rel),
                message: "non-Markdown file ignored by OKF validation".to_string(),
            });
            continue;
        }

        let abs = root.join(&rel);
        let content = std::fs::read_to_string(&abs)
            .with_context(|| format!("failed to read {}", abs.display()))?;
        validate_markdown_file(&mut report, &rel, &content);
    }

    if !report.index_present {
        report.warnings.push(OkfIssue {
            path: Some(PathBuf::from("index.md")),
            message: "root index.md is recommended for OKF navigation".to_string(),
        });
    }
    if report.concept_count == 0 {
        report.errors.push(OkfIssue {
            path: None,
            message: "OKF bundle must contain at least one concept Markdown file".to_string(),
        });
    }

    report.files.sort_by(|a, b| a.path.cmp(&b.path));
    Ok(report)
}

pub fn validate_skill_directory(source_dir: &Path) -> Result<SkillDirectoryReport> {
    let mut report = SkillDirectoryReport {
        root: source_dir.to_path_buf(),
        okf: None,
        resource_refs: Vec::new(),
        errors: Vec::new(),
        warnings: Vec::new(),
    };

    let skill_path = source_dir.join("SKILL.md");
    if !skill_path.is_file() {
        report.errors.push(OkfIssue {
            path: Some(PathBuf::from("SKILL.md")),
            message: "skill directory must contain SKILL.md".to_string(),
        });
        return Ok(report);
    }

    let skill_content = std::fs::read_to_string(&skill_path)
        .with_context(|| format!("failed to read {}", skill_path.display()))?;
    validate_skill_frontmatter(&mut report, &skill_content);

    report.resource_refs = collect_local_resource_refs(&skill_content);
    validate_resource_refs(&mut report, source_dir);
    collect_duplication_warnings(&mut report, source_dir, &skill_content)?;

    let okf_dir = source_dir.join("okf");
    if okf_dir.exists() {
        let okf_report = validate_okf_bundle(&okf_dir)?;
        report.errors.extend(
            okf_report
                .errors
                .iter()
                .map(|issue| prefix_issue("okf", issue)),
        );
        report.warnings.extend(
            okf_report
                .warnings
                .iter()
                .map(|issue| prefix_issue("okf", issue)),
        );
        report.okf = Some(okf_report);
    }

    Ok(report)
}

pub fn validate_skill_directory_okf(source_dir: &Path) -> Result<()> {
    let report = validate_skill_directory(source_dir)?;
    for message in report.warning_messages() {
        eprintln!("warning: {message}");
    }
    if report.is_valid() {
        return Ok(());
    }

    anyhow::bail!(
        "invalid skill directory at {}:\n{}",
        source_dir.display(),
        report.error_messages().join("\n")
    );
}

pub fn single_file_resource_warnings(file: &Path, content: &str) -> Vec<String> {
    collect_local_resource_refs(content)
        .into_iter()
        .map(|resource| {
            format!(
                "{}:{} references local resource `{}`; use `install-dir` to copy companion resources",
                file.display(),
                resource.line,
                resource.path.display()
            )
        })
        .collect()
}

fn validate_skill_frontmatter(report: &mut SkillDirectoryReport, content: &str) {
    let frontmatter = match split_frontmatter(content) {
        Ok((frontmatter, _)) => frontmatter,
        Err(message) => {
            report.errors.push(OkfIssue {
                path: Some(PathBuf::from("SKILL.md")),
                message,
            });
            return;
        }
    };

    let Some(raw) = frontmatter else {
        return;
    };

    let value = match serde_yaml::from_str::<Value>(raw) {
        Ok(value) => value,
        Err(err) => {
            report.errors.push(OkfIssue {
                path: Some(PathBuf::from("SKILL.md")),
                message: format!("invalid YAML frontmatter: {err}"),
            });
            return;
        }
    };

    let Some(mapping) = value.as_mapping() else {
        report.errors.push(OkfIssue {
            path: Some(PathBuf::from("SKILL.md")),
            message: "frontmatter must be a YAML mapping".to_string(),
        });
        return;
    };

    validate_dynamic_context(report, mapping);
}

fn validate_dynamic_context(report: &mut SkillDirectoryReport, mapping: &Mapping) {
    let Some(value) = mapping.get(Value::String("dynamic_context".to_string())) else {
        return;
    };

    let Some(items) = value.as_sequence() else {
        report.errors.push(OkfIssue {
            path: Some(PathBuf::from("SKILL.md")),
            message: "`dynamic_context` must be a YAML sequence".to_string(),
        });
        return;
    };

    for (index, item) in items.iter().enumerate() {
        let Some(item_mapping) = item.as_mapping() else {
            report.errors.push(OkfIssue {
                path: Some(PathBuf::from("SKILL.md")),
                message: format!("`dynamic_context[{index}]` must be a YAML mapping"),
            });
            continue;
        };

        validate_dynamic_context_string_field(report, item_mapping, index, "name", true);
        validate_dynamic_context_string_field(report, item_mapping, index, "command", true);
        validate_dynamic_context_string_field(report, item_mapping, index, "cache_owner", false);

        if let Some(command) = string_field(item_mapping, "command")
            && command.lines().count() > 1
        {
            report.errors.push(OkfIssue {
                path: Some(PathBuf::from("SKILL.md")),
                message: format!("`dynamic_context[{index}].command` must be a single-line string"),
            });
        }
    }
}

fn validate_dynamic_context_string_field(
    report: &mut SkillDirectoryReport,
    mapping: &Mapping,
    index: usize,
    field: &str,
    required: bool,
) {
    let Some(value) = mapping.get(Value::String(field.to_string())) else {
        if required {
            report.errors.push(OkfIssue {
                path: Some(PathBuf::from("SKILL.md")),
                message: format!("`dynamic_context[{index}].{field}` is required"),
            });
        }
        return;
    };

    match scalar_to_string(value) {
        Some(value) if !value.trim().is_empty() => {}
        Some(_) => report.errors.push(OkfIssue {
            path: Some(PathBuf::from("SKILL.md")),
            message: format!("`dynamic_context[{index}].{field}` must not be empty"),
        }),
        None => report.errors.push(OkfIssue {
            path: Some(PathBuf::from("SKILL.md")),
            message: format!("`dynamic_context[{index}].{field}` must be a string"),
        }),
    }
}

fn collect_local_resource_refs(content: &str) -> Vec<ResourceRef> {
    let mut refs = BTreeSet::new();
    for (line_index, line) in content.lines().enumerate() {
        let line_number = line_index + 1;
        for target in markdown_link_targets(line) {
            if let Some(path) = normalize_resource_ref(target, true) {
                refs.insert(ResourceRef {
                    path,
                    line: line_number,
                });
            }
        }
        for token in path_like_tokens(line) {
            if let Some(path) = normalize_resource_ref(token, false) {
                refs.insert(ResourceRef {
                    path,
                    line: line_number,
                });
            }
        }
    }
    refs.into_iter().collect()
}

fn markdown_link_targets(line: &str) -> Vec<&str> {
    let mut targets = Vec::new();
    let mut rest = line;
    while let Some(start) = rest.find("](") {
        let after_open = &rest[start + 2..];
        let Some(end) = after_open.find(')') else {
            break;
        };
        targets.push(&after_open[..end]);
        rest = &after_open[end + 1..];
    }
    targets
}

fn path_like_tokens(line: &str) -> impl Iterator<Item = &str> {
    line.split(|ch: char| {
        ch.is_whitespace()
            || matches!(
                ch,
                '(' | ')' | '[' | ']' | '{' | '}' | '<' | '>' | '"' | '\'' | '`' | ',' | ';'
            )
    })
}

fn normalize_resource_ref(raw: &str, allow_spec: bool) -> Option<PathBuf> {
    let mut value = raw.trim();
    if value.is_empty()
        || value.starts_with('#')
        || value.starts_with("http://")
        || value.starts_with("https://")
        || value.starts_with("mailto:")
        || value.starts_with("skill://")
        || value.starts_with('/')
    {
        return None;
    }

    value = value
        .split(['#', '?'])
        .next()
        .unwrap_or(value)
        .trim_matches(|ch: char| matches!(ch, ':' | '.' | '!' | '?'));
    while let Some(stripped) = value.strip_prefix("./") {
        value = stripped;
    }

    if (allow_spec && value == "SPEC.md")
        || RESOURCE_ROOTS.iter().any(|root| {
            let prefix = format!("{root}/");
            value.starts_with(&prefix) && value.len() > prefix.len()
        })
    {
        return Some(PathBuf::from(value));
    }

    None
}

fn validate_resource_refs(report: &mut SkillDirectoryReport, source_dir: &Path) {
    for resource in report.resource_refs.clone() {
        if resource
            .path
            .components()
            .any(|component| matches!(component, std::path::Component::ParentDir))
        {
            report.errors.push(OkfIssue {
                path: Some(PathBuf::from("SKILL.md")),
                message: format!(
                    "line {} references resource outside the skill directory: {}",
                    resource.line,
                    resource.path.display()
                ),
            });
            continue;
        }

        if !source_dir.join(&resource.path).exists() {
            report.errors.push(OkfIssue {
                path: Some(PathBuf::from("SKILL.md")),
                message: format!(
                    "line {} references missing resource: {}",
                    resource.line,
                    resource.path.display()
                ),
            });
        }
    }
}

fn collect_duplication_warnings(
    report: &mut SkillDirectoryReport,
    source_dir: &Path,
    skill_content: &str,
) -> Result<()> {
    let skill_lines: BTreeSet<String> = skill_content
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty())
        .map(ToOwned::to_owned)
        .collect();
    let skill_code_blocks = fenced_code_blocks(skill_content);
    let mut checked = BTreeSet::new();

    for resource in report.resource_refs.clone() {
        if !checked.insert(resource.path.clone()) {
            continue;
        }
        let path = source_dir.join(&resource.path);
        if !path.is_file() {
            continue;
        }

        let content = std::fs::read_to_string(&path)
            .with_context(|| format!("failed to read {}", path.display()))?;

        if resource.path.extension().and_then(|ext| ext.to_str()) == Some("md") {
            collect_markdown_duplication_warnings(report, &resource.path, skill_content, &content);
        } else if path_starts_with(&resource.path, "scripts") {
            collect_script_duplication_warnings(
                report,
                &resource.path,
                &skill_lines,
                &skill_code_blocks,
                &content,
            );
        }
    }

    Ok(())
}

fn collect_markdown_duplication_warnings(
    report: &mut SkillDirectoryReport,
    resource_path: &Path,
    skill_content: &str,
    resource_content: &str,
) {
    for heading in resource_content
        .lines()
        .map(str::trim)
        .filter(|line| line.starts_with('#') && line.len() >= 12)
    {
        if skill_content.lines().any(|line| line.trim() == heading) {
            report.warnings.push(OkfIssue {
                path: Some(PathBuf::from("SKILL.md")),
                message: format!(
                    "SKILL.md appears to duplicate heading `{heading}` from {}; keep details in the resource and route to it",
                    resource_path.display()
                ),
            });
        }
    }

    for line in resource_content
        .lines()
        .map(str::trim)
        .filter(|line| line.len() >= 100)
    {
        if skill_content.contains(line) {
            report.warnings.push(OkfIssue {
                path: Some(PathBuf::from("SKILL.md")),
                message: format!(
                    "SKILL.md appears to duplicate a long passage from {}; keep bulky context in the resource",
                    resource_path.display()
                ),
            });
            return;
        }
    }
}

fn collect_script_duplication_warnings(
    report: &mut SkillDirectoryReport,
    resource_path: &Path,
    skill_lines: &BTreeSet<String>,
    skill_code_blocks: &[String],
    script_content: &str,
) {
    let script_lines: Vec<String> = script_content
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty())
        .map(ToOwned::to_owned)
        .collect();
    if script_lines.len() < 3 {
        return;
    }

    let prefix = script_lines
        .iter()
        .take(3)
        .cloned()
        .collect::<Vec<_>>()
        .join("\n");
    let duplicate_prefix = skill_code_blocks
        .iter()
        .any(|block| block.contains(&prefix));
    let duplicate_lines = script_lines
        .iter()
        .take(5)
        .filter(|line| skill_lines.contains(*line))
        .count()
        >= 3;

    if duplicate_prefix || duplicate_lines {
        report.warnings.push(OkfIssue {
            path: Some(PathBuf::from("SKILL.md")),
            message: format!(
                "SKILL.md appears to duplicate script content from {}; keep executable logic in scripts/",
                resource_path.display()
            ),
        });
    }
}

fn fenced_code_blocks(content: &str) -> Vec<String> {
    let mut blocks = Vec::new();
    let mut current = Vec::new();
    let mut in_block = false;

    for line in content.lines() {
        if line.trim_start().starts_with("```") {
            if in_block {
                blocks.push(current.join("\n"));
                current.clear();
            }
            in_block = !in_block;
            continue;
        }

        if in_block {
            current.push(line.trim().to_string());
        }
    }

    blocks
}

fn path_starts_with(path: &Path, prefix: &str) -> bool {
    path.components()
        .next()
        .and_then(|component| match component {
            std::path::Component::Normal(value) => value.to_str(),
            _ => None,
        })
        == Some(prefix)
}

fn prefix_issue(prefix: &str, issue: &OkfIssue) -> OkfIssue {
    OkfIssue {
        path: issue
            .path
            .as_ref()
            .map(|path| PathBuf::from(prefix).join(path))
            .or_else(|| Some(PathBuf::from(prefix))),
        message: issue.message.clone(),
    }
}

fn validate_markdown_file(report: &mut OkfReport, rel: &Path, content: &str) {
    let kind = match rel.to_string_lossy().replace('\\', "/").as_str() {
        "index.md" => {
            report.index_present = true;
            OkfFileKind::Index
        }
        "log.md" => {
            report.log_present = true;
            OkfFileKind::Log
        }
        _ => OkfFileKind::Concept,
    };

    let (frontmatter, body) = match split_frontmatter(content) {
        Ok(parts) => parts,
        Err(message) => {
            report.errors.push(OkfIssue {
                path: Some(rel.to_path_buf()),
                message,
            });
            (None, content)
        }
    };

    let yaml = frontmatter.and_then(|raw| parse_frontmatter(report, rel, raw));
    let title = markdown_title(body);
    let mut concept_type = None;
    let mut tags = Vec::new();

    if let Some(mapping) = yaml.as_ref().and_then(Value::as_mapping) {
        if kind == OkfFileKind::Index {
            report.okf_version = string_field(mapping, "okf_version");
        }
        concept_type = string_field(mapping, "type");
        tags = string_or_sequence_field(mapping, "tags");
    }

    if kind == OkfFileKind::Concept {
        report.concept_count += 1;
        match yaml {
            None => report.errors.push(OkfIssue {
                path: Some(rel.to_path_buf()),
                message: "concept file must start with YAML frontmatter".to_string(),
            }),
            Some(Value::Mapping(_)) => {
                if concept_type.as_deref().unwrap_or("").trim().is_empty() {
                    report.errors.push(OkfIssue {
                        path: Some(rel.to_path_buf()),
                        message: "concept frontmatter must include non-empty `type`".to_string(),
                    });
                }
            }
            Some(_) => report.errors.push(OkfIssue {
                path: Some(rel.to_path_buf()),
                message: "frontmatter must be a YAML mapping".to_string(),
            }),
        }
    }

    report.files.push(OkfFile {
        path: rel.to_path_buf(),
        kind,
        concept_type,
        title,
        tags,
    });
}

fn parse_frontmatter(report: &mut OkfReport, rel: &Path, raw: &str) -> Option<Value> {
    match serde_yaml::from_str::<Value>(raw) {
        Ok(value) => Some(value),
        Err(err) => {
            report.errors.push(OkfIssue {
                path: Some(rel.to_path_buf()),
                message: format!("invalid YAML frontmatter: {err}"),
            });
            None
        }
    }
}

fn string_field(mapping: &Mapping, field: &str) -> Option<String> {
    mapping
        .get(Value::String(field.to_string()))
        .and_then(scalar_to_string)
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
}

fn scalar_to_string(value: &Value) -> Option<String> {
    match value {
        Value::String(value) => Some(value.clone()),
        Value::Number(value) => Some(value.to_string()),
        _ => None,
    }
}

fn string_or_sequence_field(mapping: &Mapping, field: &str) -> Vec<String> {
    let Some(value) = mapping.get(Value::String(field.to_string())) else {
        return Vec::new();
    };
    if let Some(single) = value.as_str() {
        let trimmed = single.trim();
        return (!trimmed.is_empty())
            .then(|| trimmed.to_string())
            .into_iter()
            .collect();
    }
    value
        .as_sequence()
        .map(|items| {
            items
                .iter()
                .filter_map(Value::as_str)
                .map(str::trim)
                .filter(|value| !value.is_empty())
                .map(ToOwned::to_owned)
                .collect()
        })
        .unwrap_or_default()
}

fn split_frontmatter(content: &str) -> std::result::Result<(Option<&str>, &str), String> {
    let mut cursor = 0;
    let mut lines = content.split_inclusive('\n');
    let Some(first) = lines.next() else {
        return Ok((None, content));
    };
    if trim_line_ending(first) != "---" {
        return Ok((None, content));
    }

    let start = first.len();
    cursor += first.len();
    for line in lines {
        if trim_line_ending(line) == "---" {
            let frontmatter = &content[start..cursor];
            let body = &content[cursor + line.len()..];
            return Ok((Some(frontmatter), body));
        }
        cursor += line.len();
    }

    if content[cursor..].trim() == "---" {
        let frontmatter = &content[start..cursor];
        return Ok((Some(frontmatter), ""));
    }

    Err("frontmatter opened with `---` but no closing delimiter was found".to_string())
}

fn trim_line_ending(line: &str) -> &str {
    line.trim_end_matches('\n').trim_end_matches('\r')
}

fn markdown_title(body: &str) -> Option<String> {
    body.lines()
        .find_map(|line| line.strip_prefix("# ").map(str::trim))
        .filter(|title| !title.is_empty())
        .map(ToOwned::to_owned)
}

fn collect_relative_files(root: &Path) -> Result<BTreeSet<PathBuf>> {
    let mut files = BTreeSet::new();
    collect_relative_files_inner(root, root, &mut files)?;
    Ok(files)
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn accepts_valid_okf_bundle() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(
            dir.path().join("index.md"),
            "---\nokf_version: 0.1\n---\n# Index\n",
        )
        .unwrap();
        std::fs::write(
            dir.path().join("concept.md"),
            "---\ntype: concept\ntags: [agent, context]\n---\n# Concept\nBody.\n",
        )
        .unwrap();

        let report = validate_okf_bundle(dir.path()).unwrap();
        assert!(report.is_valid(), "{:?}", report.error_messages());
        assert_eq!(report.okf_version.as_deref(), Some("0.1"));
        assert_eq!(report.concept_count, 1);
        assert_eq!(report.files.len(), 2);
    }

    #[test]
    fn rejects_concept_without_type() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(
            dir.path().join("concept.md"),
            "---\ntags: [agent]\n---\n# Concept\n",
        )
        .unwrap();

        let report = validate_okf_bundle(dir.path()).unwrap();
        assert!(!report.is_valid());
        assert!(
            report
                .error_messages()
                .iter()
                .any(|message| message.contains("non-empty `type`"))
        );
    }

    #[test]
    fn validates_skill_directory_okf_subdir() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("SKILL.md"), "# Skill\n").unwrap();
        std::fs::create_dir_all(dir.path().join("okf")).unwrap();
        std::fs::write(
            dir.path().join("okf/context.md"),
            "---\ntype: reference\n---\n# Context\n",
        )
        .unwrap();

        validate_skill_directory_okf(dir.path()).unwrap();
    }

    #[test]
    fn reports_missing_linked_runbook() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(
            dir.path().join("SKILL.md"),
            "# Skill\n\nFollow [Deploy](runbooks/deploy.md).\n",
        )
        .unwrap();

        let report = validate_skill_directory(dir.path()).unwrap();
        assert!(!report.is_valid());
        assert!(report.error_messages().iter().any(|message| {
            message.contains("missing resource") && message.contains("runbooks/deploy.md")
        }));
    }

    #[test]
    fn accepts_existing_reference_and_ignores_external_links() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::create_dir_all(dir.path().join("references")).unwrap();
        std::fs::write(dir.path().join("references/schema.md"), "# Schema\n").unwrap();
        std::fs::write(
            dir.path().join("SKILL.md"),
            "# Skill\n\nRead [schema](references/schema.md) and https://example.com/runbooks/nope.md.\n",
        )
        .unwrap();

        let report = validate_skill_directory(dir.path()).unwrap();
        assert!(report.is_valid(), "{:?}", report.error_messages());
        assert_eq!(report.resource_refs.len(), 1);
    }

    #[test]
    fn single_file_install_warns_about_resource_links() {
        let warnings = single_file_resource_warnings(
            Path::new("SKILL.md"),
            "# Skill\n\nUse `runbooks/deploy.md` when deploying.\n",
        );

        assert_eq!(warnings.len(), 1);
        assert!(warnings[0].contains("install-dir"));
        assert!(warnings[0].contains("runbooks/deploy.md"));
    }

    #[test]
    fn bare_resource_root_mentions_are_not_file_references() {
        let warnings = single_file_resource_warnings(
            Path::new("SKILL.md"),
            "# Skill\n\nUse `install-dir` for skills that include `runbooks/`, `okf/`, or `assets/`.\n",
        );

        assert!(warnings.is_empty());
    }

    #[test]
    fn bare_spec_mentions_are_not_file_references() {
        let warnings = single_file_resource_warnings(
            Path::new("SKILL.md"),
            "# Skill\n\nUse `install-dir` for skills that include `SPEC.md`.\n",
        );

        assert!(warnings.is_empty());
    }

    #[test]
    fn linked_spec_mentions_are_validated() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(
            dir.path().join("SKILL.md"),
            "# Skill\n\nRead [the spec](SPEC.md) before editing.\n",
        )
        .unwrap();

        let report = validate_skill_directory(dir.path()).unwrap();
        assert!(!report.is_valid());
        assert!(
            report
                .error_messages()
                .iter()
                .any(|message| message.contains("missing resource") && message.contains("SPEC.md"))
        );
    }

    #[test]
    fn valid_dynamic_context_metadata_passes() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(
            dir.path().join("SKILL.md"),
            "---\ndynamic_context:\n  - name: code-context\n    command: tsift --envelope context-pack {query} --budget normal\n    cache_owner: tsift\n---\n# Skill\n",
        )
        .unwrap();

        let report = validate_skill_directory(dir.path()).unwrap();
        assert!(report.is_valid(), "{:?}", report.error_messages());
    }

    #[test]
    fn invalid_dynamic_context_command_shape_fails() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(
            dir.path().join("SKILL.md"),
            "---\ndynamic_context:\n  - name: code-context\n    command: [tsift, context-pack]\n---\n# Skill\n",
        )
        .unwrap();

        let report = validate_skill_directory(dir.path()).unwrap();
        assert!(!report.is_valid());
        assert!(report.error_messages().iter().any(|message| {
            message.contains("dynamic_context[0].command") && message.contains("string")
        }));
    }

    #[test]
    fn duplicated_runbook_heading_emits_warning() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::create_dir_all(dir.path().join("runbooks")).unwrap();
        std::fs::write(
            dir.path().join("runbooks/deploy.md"),
            "# Deploy\n\n## Rollout Steps\n\nRun the deploy command after checking health.\n",
        )
        .unwrap();
        std::fs::write(
            dir.path().join("SKILL.md"),
            "# Skill\n\nUse `runbooks/deploy.md`.\n\n## Rollout Steps\n\nRun the deploy command after checking health.\n",
        )
        .unwrap();

        let report = validate_skill_directory(dir.path()).unwrap();
        assert!(report.is_valid(), "{:?}", report.error_messages());
        assert!(report.warning_messages().iter().any(|message| {
            message.contains("duplicate heading") && message.contains("runbooks/deploy.md")
        }));
    }

    #[test]
    fn short_router_sentence_does_not_warn() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::create_dir_all(dir.path().join("runbooks")).unwrap();
        std::fs::write(
            dir.path().join("runbooks/deploy.md"),
            "# Deploy\n\n## Rollout Steps\n\nRun the deploy command after checking health.\n",
        )
        .unwrap();
        std::fs::write(
            dir.path().join("SKILL.md"),
            "# Skill\n\nUse `runbooks/deploy.md` for deploys.\n",
        )
        .unwrap();

        let report = validate_skill_directory(dir.path()).unwrap();
        assert!(report.is_valid(), "{:?}", report.error_messages());
        assert!(report.warning_messages().is_empty());
    }
}
