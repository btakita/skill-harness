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
    pub missing_guidance: Vec<&'static str>,
}

impl CompositionPlanReport {
    pub fn is_valid(&self) -> bool {
        self.missing_sections.is_empty()
            && !self.candidate_names.is_empty()
            && self.missing_guidance.is_empty()
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
        messages.extend(
            self.missing_guidance
                .iter()
                .map(|message| message.to_string()),
        );
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
    let missing_guidance = missing_guidance(content);

    CompositionPlanReport {
        missing_sections,
        candidate_names,
        missing_guidance,
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

fn missing_guidance(content: &str) -> Vec<&'static str> {
    let mut messages = Vec::new();
    if !decision_boundary_has_one_vs_many_rationale(content) {
        messages.push(
            "decision boundary must explain the one-skill-vs-many rationale with explicit keep/split criteria",
        );
    }
    if mentions_skill_creation(content) && !has_skill_creator_handoff_boundary(content) {
        messages.push(
            "skill-creator handoff boundary missing; name when compose-skills stops and skill-creator begins",
        );
    }
    messages
}

fn decision_boundary_has_one_vs_many_rationale(content: &str) -> bool {
    let Some(body) = section_body(content, "Decision Boundary") else {
        return true;
    };
    let lower = body.to_ascii_lowercase();
    let one_skill_language = lower.contains("one skill")
        || lower.contains("single skill")
        || lower.contains("keep it as one")
        || lower.contains("keep one skill");
    let split_language = lower.contains("split")
        || lower.contains("many")
        || lower.contains("several")
        || lower.contains("larger than one")
        || lower.contains("separate");
    one_skill_language && split_language
}

fn mentions_skill_creation(content: &str) -> bool {
    let lower = content.to_ascii_lowercase();
    lower.contains("skill-creator") || lower.contains("skill creation")
}

fn has_skill_creator_handoff_boundary(content: &str) -> bool {
    let lower = content.to_ascii_lowercase();
    lower.contains("skill-creator") && (lower.contains("handoff") || lower.contains("hand off"))
}

fn section_body<'a>(content: &'a str, title: &str) -> Option<&'a str> {
    let mut in_section = false;
    let mut start = 0;
    let mut end = content.len();

    for line in content.lines() {
        let line_start = line.as_ptr() as usize - content.as_ptr() as usize;
        if let Some(heading) = markdown_heading(line) {
            if in_section {
                end = line_start;
                break;
            }
            if heading == title {
                in_section = true;
                start = line_start + line.len();
            }
        }
    }

    in_section.then(|| content[start..end].trim())
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
    fn reports_golden_diagnostics_for_malformed_fixture() {
        let report = validate_composition_plan(include_str!(
            "../skills/compose-skills/references/fixtures/malformed-plan.md"
        ));
        assert_eq!(
            report.error_messages(),
            vec![
                "missing required section(s): Resource Inventory, Invocation Policy, Validation Plan, Recommendation",
                "no candidate skill entries found; include at least one `name: skill-name` entry",
                "decision boundary must explain the one-skill-vs-many rationale with explicit keep/split criteria",
            ]
        );
    }

    #[test]
    fn reports_missing_one_skill_vs_many_rationale() {
        let report = validate_composition_plan(
            r#"
## Decision Boundary
Create this skill because it sounds useful.

## Proposed Skills
- name: useful-skill

## Resource Inventory
Use SKILL.md.

## Invocation Policy
Implicit.

## Validation Plan
Forward-test trigger prompts.

## Recommendation
Proceed.
"#,
        );
        assert_eq!(
            report.error_messages(),
            vec![
                "decision boundary must explain the one-skill-vs-many rationale with explicit keep/split criteria",
            ]
        );
    }

    #[test]
    fn reports_missing_skill_creator_handoff_boundary() {
        let report = validate_composition_plan(
            r#"
## Decision Boundary
Keep one skill until there is enough reuse to split into several skills.

## Proposed Skills
- name: compose-skills
  purpose: Plan skills before implementation.
- name: skill-creator
  purpose: Implement the approved skill.

## Resource Inventory
Use SKILL.md and templates.

## Invocation Policy
Implicit.

## Validation Plan
Forward-test chained prompts.

## Recommendation
Proceed with both skills.
"#,
        );
        assert_eq!(
            report.error_messages(),
            vec![
                "skill-creator handoff boundary missing; name when compose-skills stops and skill-creator begins",
            ]
        );
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

    #[test]
    fn fixture_rejects_malformed_plan() {
        let report = validate_composition_plan(include_str!(
            "../skills/compose-skills/references/fixtures/malformed-plan.md"
        ));
        assert!(!report.is_valid());
        assert!(report.candidate_names.is_empty());
        assert!(report.missing_sections.contains(&"Resource Inventory"));
    }

    #[test]
    fn fixture_accepts_ambiguous_one_vs_many_split() {
        let report = validate_composition_plan(include_str!(
            "../skills/compose-skills/references/fixtures/ambiguous-one-vs-many.md"
        ));
        assert!(report.is_valid());
        assert_eq!(
            report.candidate_names,
            BTreeSet::from(["documentation-cleanup".to_string()])
        );
    }

    #[test]
    fn fixture_accepts_agent_doc_workflow_decomposition() {
        let report = validate_composition_plan(include_str!(
            "../skills/compose-skills/references/fixtures/agent-doc-decomposition.md"
        ));
        assert!(report.is_valid());
        assert_eq!(
            report.candidate_names,
            BTreeSet::from([
                "agent-doc-route-diagnostics".to_string(),
                "agent-doc-session".to_string()
            ])
        );
    }

    #[test]
    fn fixture_accepts_oversized_workflow_handoff_example() {
        let report = validate_composition_plan(include_str!(
            "../skills/compose-skills/references/fixtures/oversized-workflow-handoff.md"
        ));
        assert!(report.is_valid());
        assert_eq!(
            report.candidate_names,
            BTreeSet::from([
                "customer-research".to_string(),
                "launch-copy".to_string(),
                "skill-creator".to_string()
            ])
        );
    }
}
