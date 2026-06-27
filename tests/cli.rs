use std::process::Command;

fn skill_harness() -> Command {
    Command::new(env!("CARGO_BIN_EXE_skill-harness"))
}

#[test]
fn install_file_warns_on_local_resource_links() {
    let source = tempfile::tempdir().unwrap();
    let skill = source.path().join("SKILL.md");
    std::fs::write(
        &skill,
        "# Context Skill\n\nUse `runbooks/context.md` before answering.\n",
    )
    .unwrap();

    let project = tempfile::tempdir().unwrap();
    let output = skill_harness()
        .args([
            "install",
            "context-skill",
            "--file",
            skill.to_str().unwrap(),
            "--harness",
            "generic",
            "--root",
            project.path().to_str().unwrap(),
        ])
        .output()
        .unwrap();

    assert!(
        output.status.success(),
        "stdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("warning:"));
    assert!(stderr.contains("runbooks/context.md"));
    assert!(stderr.contains("install-dir"));
    assert!(
        project
            .path()
            .join(".agent/skills/context-skill/SKILL.md")
            .is_file()
    );
    assert!(
        !project
            .path()
            .join(".agent/skills/context-skill/runbooks/context.md")
            .exists()
    );
}

#[test]
fn okf_validate_reports_invalid_bundle() {
    let bundle = tempfile::tempdir().unwrap();
    std::fs::write(bundle.path().join("concept.md"), "# Missing Frontmatter\n").unwrap();

    let output = skill_harness()
        .args(["okf", "validate", bundle.path().to_str().unwrap()])
        .output()
        .unwrap();

    assert!(!output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stdout.contains("OKF bundle:"));
    assert!(stderr.contains("concept file must start with YAML frontmatter"));
}
