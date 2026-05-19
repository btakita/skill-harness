# skill-harness

Manage AI agent skills — install, check, uninstall, and list skills across environments.

## Invocation

```
skill-harness install <name> --file <SKILL.md>
skill-harness install-dir <name> --source <skill-directory> --harness <target>
skill-harness check <name> --file <SKILL.md>
skill-harness check-dir <name> --source <skill-directory> --harness <target>
skill-harness uninstall <name>
skill-harness list
skill-harness compose validate <plan.md>
```

## When to use

- When asked to install a skill into a project
- When checking if a skill is up to date
- When removing a skill from the project
- When listing all installed skills

## Commands

### install

Install a skill to the appropriate environment-specific path.

```bash
skill-harness install email --file .agent/skills/email/SKILL.md
```

The target path depends on the detected environment:
- Claude Code: `.claude/skills/<name>/SKILL.md`
- OpenCode: `.opencode/skills/<name>/SKILL.md`
- Codex: `.codex/skills/<name>/SKILL.md` when `--harness codex` is explicit
- Cursor: `.cursor/rules/<name>.md`
- Generic: `.agent/skills/<name>/SKILL.md`

Use `install-dir` when the skill has companion files such as `SPEC.md`, `references/`, `runbooks/`, or `assets/`.

```bash
skill-harness install-dir compose-skills --harness claude
skill-harness install-dir compose-skills --harness codex
skill-harness install-dir compose-skills --harness opencode
skill-harness install-dir compose-skills --harness generic
```

### check

Verify if an installed skill matches the source content.

```bash
skill-harness check email --file .agent/skills/email/SKILL.md
```

Returns exit code 0 if up to date, 1 if outdated or not installed.

Use `check-dir` for skills with companion files. For `compose-skills`, the bundled `skills/compose-skills` directory is the canonical source and does not need `--source`.

```bash
skill-harness check-dir compose-skills --harness claude
skill-harness check-dir compose-skills --harness codex
```

### uninstall

Remove a skill from the current environment.

```bash
skill-harness uninstall email
```

Removes the skill file and cleans up empty parent directories.

### list

Show all installed skills across known locations.

```bash
skill-harness list
```

Scans `.agent/skills/` and environment-specific skill directories.

### compose validate

Validate a compose-skills architecture plan with the Rust validator.

```bash
skill-harness compose validate skills/compose-skills/references/example-plan.md
```

## Runbooks

- `install skill` — [runbooks/install-skill.md](runbooks/install-skill.md)
