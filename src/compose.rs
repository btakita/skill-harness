//! Composition-plan validation for reusable agent skill systems.

use anyhow::{Context, Result};
use std::collections::BTreeSet;
use std::path::Path;

pub const REQUIRED_SECTIONS: &[&str] = &[
    "Decision Boundary",
    "Proposed Skills",
    "Resource Inventory",
    "Invocation Policy",
    "Validation Plan",
    "Recommendation",
];

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CompositionPlanReport {
    pub missing_sections: Vec<&'static str>,
    pub candidate_names: BTreeSet<String>,
}

impl CompositionPlanReport {
    pub fn is_valid(&self) -> bool {
        self.missing_sections.is_empty() && !self.candidate_names.is_empty()
    }

    pub fn error_messages(&self) -> Vec<String> {
        let mut messages = Vec::new();
        if !self.missing_sections.is_empty() {
            messages.push(format!(
                "missing required section(s): {}",
                self.missing_sections.join(", ")
            ));
        }
        if self.candidate_names.is_empty() {
            messages.push(
                "no candidate skill entries found; include at least one `name: skill-name` entry"
                    .to_string(),
            );
        }
        messages
    }
}

pub fn validate_composition_plan_path(path: &Path) -> Result<CompositionPlanReport> {
    let content = std::fs::read_to_string(path)
        .with_context(|| format!("failed to read {}", path.display()))?;
    Ok(validate_composition_plan(&content))
}

pub fn validate_composition_plan(content: &str) -> CompositionPlanReport {
    let headings: BTreeSet<String> = content.lines().filter_map(markdown_heading).collect();
    let missing_sections = REQUIRED_SECTIONS
        .iter()
        .copied()
        .filter(|section| !headings.contains(*section))
        .collect();
    let candidate_names = content.lines().filter_map(candidate_skill_name).collect();

    CompositionPlanReport {
        missing_sections,
        candidate_names,
    }
}

fn markdown_heading(line: &str) -> Option<String> {
    let trimmed = line.trim();
    if !trimmed.starts_with('#') {
        return None;
    }
    let level = trimmed.chars().take_while(|c| *c == '#').count();
    if level < 2 {
        return None;
    }
    let title = trimmed[level..].trim();
    if title.is_empty() {
        return None;
    }
    Some(title.to_string())
}

fn candidate_skill_name(line: &str) -> Option<String> {
    let trimmed = line.trim_start();
    let is_candidate_line = trimmed.starts_with('-') || trimmed.starts_with('#');
    if !is_candidate_line {
        return None;
    }

    let lower = trimmed.to_ascii_lowercase();
    let start = lower.find("name:")? + "name:".len();
    let after_name = trimmed[start..].trim_start().trim_start_matches('`');
    let name: String = after_name
        .chars()
        .take_while(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || *c == '-')
        .collect();

    if is_hyphen_case_skill_name(&name) {
        Some(name)
    } else {
        None
    }
}

fn is_hyphen_case_skill_name(name: &str) -> bool {
    let parts: Vec<&str> = name.split('-').collect();
    parts.len() >= 2
        && parts.iter().all(|part| {
            !part.is_empty()
                && part
                    .chars()
                    .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit())
        })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn valid_plan() -> &'static str {
        r#"
## Decision Boundary
Keep one skill until reuse or safety needs a split.

## Proposed Skills
- name: compose-skills
  purpose: Design small skill systems.

## Resource Inventory
Use references and scripts where they remove repeated reasoning.

## Invocation Policy
The planning skill is implicit.

## Validation Plan
Run positive, chained, and negative trigger checks.

## Recommendation
Create the standalone planning skill.
"#
    }

    #[test]
    fn accepts_valid_composition_plan() {
        let report = validate_composition_plan(valid_plan());
        assert!(report.is_valid());
        assert_eq!(
            report.candidate_names,
            BTreeSet::from(["compose-skills".to_string()])
        );
    }

    #[test]
    fn reports_missing_sections_and_missing_candidates() {
        let report = validate_composition_plan("## Decision Boundary\n\nNo candidates.\n");
        assert!(!report.is_valid());
        assert_eq!(
            report.missing_sections,
            vec![
                "Proposed Skills",
                "Resource Inventory",
                "Invocation Policy",
                "Validation Plan",
                "Recommendation"
            ]
        );
        assert!(report.candidate_names.is_empty());
    }

    #[test]
    fn requires_hyphen_case_candidate_names() {
        let report = validate_composition_plan(
            "## Proposed Skills\n- name: skill\n- name: ComposeSkills\n- name: compose-skills\n",
        );
        assert_eq!(
            report.candidate_names,
            BTreeSet::from(["compose-skills".to_string()])
        );
    }
}
