# skill-harness spec

Format specification for contextually-activated instruction bundles. Skills package rules, runbooks, and examples into self-contained directories with activation metadata.

## Skill Directory Structure

```
.agent/skills/<name>/
├── SKILL.md           # required: instruction content + activation metadata
├── runbooks/          # optional: on-demand procedures
│   ├── deploy.md
│   └── migrate.md
└── examples/          # optional: reference material
    ├── config.yaml
    └── template.ts
```

The skill name is the directory name. It should be a lowercase slug (e.g., `testing`, `deploy`, `api-client`).

## SKILL.md Format

Each SKILL.md is a markdown file with optional YAML frontmatter:

```markdown
---
description: "One-line description for contextual activation"
globs: ["**/*.test.ts", "**/*.spec.ts"]
alwaysApply: false
---

# Skill Name

Instruction content here. This is the body that gets loaded when the skill activates.

## Rules

- Convention or constraint specific to this capability.

## Runbooks

- **Deploy**: Follow `runbooks/deploy.md` when deploying.
- **Migrate**: Follow `runbooks/migrate.md` for database migrations.
```

### Frontmatter Fields

| Field | Type | Required | Default | Description |
|-------|------|----------|---------|-------------|
| `description` | string | yes | -- | One-line summary used for agent-requested activation |
| `globs` | string[] | no | `[]` | File patterns that trigger activation |
| `alwaysApply` | boolean | no | `false` | Whether to load on every interaction |

### Body Content

The markdown body contains the skill's instruction content. It can include:

- **Rules** -- declarative policy (conventions, constraints, architecture decisions)
- **Runbook references** -- pointers to procedures in the `runbooks/` directory
- **Inline guidance** -- any other instruction content relevant to this capability

## Activation Modes

Skills activate based on their frontmatter configuration:

### Always

```yaml
---
description: "Project-wide coding conventions"
alwaysApply: true
---
```

Loaded on every interaction. Use sparingly -- this adds to base context cost.

### File-pattern

```yaml
---
description: "Testing conventions and utilities"
globs: ["**/*.test.ts", "**/*.spec.ts", "tests/**"]
---
```

Loaded when the agent is working with files matching the glob patterns. The matching semantics follow gitignore-style globs.

### Agent-requested

```yaml
---
description: "Database migration procedures and conventions"
---
```

The agent reads the `description` and decides whether to load the skill based on the current task. No `globs` or `alwaysApply` -- activation is the agent's judgment call.

### Manual

No frontmatter needed. The user explicitly tells the agent to use the skill:

> "Use the deploy skill for this."

Or references it directly:

> "Follow `.agent/skills/deploy/SKILL.md`."

## Runbooks Directory

Skills can include runbooks in a `runbooks/` subdirectory. These follow the [agent-runbooks](https://github.com/btakita/agent-runbooks) convention:

```
.agent/skills/deploy/
├── SKILL.md
└── runbooks/
    ├── production.md
    └── staging.md
```

Reference runbooks from SKILL.md:

```markdown
## Runbooks

- **Production deploy**: Follow `runbooks/production.md` for production releases.
- **Staging deploy**: Follow `runbooks/staging.md` for staging environments.
```

The trigger phrase (e.g., "Production deploy") tells the agent when to load the referenced runbook.

## Examples Directory

Skills can include reference material in an `examples/` subdirectory:

```
.agent/skills/api-client/
├── SKILL.md
└── examples/
    ├── basic-usage.ts
    └── error-handling.ts
```

Examples are not loaded by default -- the agent reads them when it needs concrete reference material.

## Install Semantics

To install a skill into a project:

1. Copy or symlink the skill directory into `.agent/skills/<name>/`
2. The skill is immediately available -- no registration step required
3. For tool-specific formats, generate the native representation:

**Claude Code:**
```bash
ln -s ../../.agent/skills/deploy .claude/skills/deploy
```

**Codex:**
```bash
skill-harness install-dir compose-skills --harness codex
skill-harness check-dir compose-skills --harness codex
```

**OpenCode:**
```bash
skill-harness install-dir compose-skills --harness opencode
skill-harness check-dir compose-skills --harness opencode
```

**Generic portable layout:**
```bash
skill-harness install-dir compose-skills --harness generic
skill-harness check-dir compose-skills --harness generic
```

For the bundled `compose-skills` package, `src/skill-harness/skills/compose-skills` is the canonical source tree. `install-dir compose-skills` and `check-dir compose-skills` use that tree by default and compare every file recursively, not only `SKILL.md`.

**Cursor (generate .mdc):**
```
---
description: "Deploy procedures and conventions"
globs:
alwaysApply: false
---
# Deploy
...
```

**Copilot (merge into scoped instructions):**
```markdown
## Deploy (from .agent/skills/deploy)
...
```

## Validation Rules

When auditing skills:

1. **SKILL.md required**: Every skill directory must contain a `SKILL.md` file
2. **Description required**: Frontmatter must include a `description` field
3. **Runbooks exist**: Any runbook referenced in SKILL.md must exist in the `runbooks/` directory
4. **No machine-local paths**: Same context invariant as other instruction files -- no `~/`, `/home/user/`, or absolute paths that won't resolve on other machines
5. **Valid frontmatter**: If YAML frontmatter is present, it must parse without errors
6. **Unique names**: No duplicate skill directory names within a project

## Composition Plan Validation

`skill-harness` also owns deterministic validation for skill-composition architecture plans. This keeps reusable skill-system planning installable without a Python runtime.

Run:

```bash
skill-harness compose validate <plan.md>
```

The validator requires these markdown sections:

- `Decision Boundary`
- `Proposed Skills`
- `Resource Inventory`
- `Invocation Policy`
- `Validation Plan`
- `Recommendation`

The plan must include at least one markdown list item or heading containing a hyphen-case candidate skill entry, such as `name: compose-skills`.

## Relationship to Other Specs

| Spec | Role in Skills |
|------|---------------|
| [agent-rules](https://github.com/btakita/agent-rules) | Rule content appears inline in SKILL.md body |
| [agent-runbooks](https://github.com/btakita/agent-runbooks) | Procedures live in the skill's `runbooks/` directory |
| [agent-memories](https://github.com/btakita/agent-memories) | Skills may generate memories during use; memories reference the skill scope |

## Agentic Contracts

Behavioral promises the agent makes when performing skill lifecycle operations. These contracts are testable via the evals below.

### Install

When installing skills, the agent promises to:

- **Install to the correct environment-specific path.** Each target environment (Claude Code, Cursor, Windsurf, etc.) has a distinct native path. The agent resolves the correct one based on the detected or specified environment.
- **Never overwrite without checking.** If a skill file already exists at the target path, the agent compares content before deciding to update. It does not silently clobber existing customizations.
- **Report the installed path.** After installation, the agent tells the user exactly where the skill was placed so they can verify.

### Check

When checking skills, the agent promises to:

- **Compare content exactly (not just version).** Staleness is determined by byte-level content comparison, not version strings or timestamps.
- **Compare full skill directories when requested.** `check-dir` reports missing, changed, and extra files relative to the canonical source directory.
- **Report outdated vs missing vs up-to-date accurately.** Each skill gets one of three statuses. The agent never conflates "missing" with "outdated" or reports false positives.

### Uninstall

When uninstalling skills, the agent promises to:

- **Remove only the skill file.** The agent deletes the specific skill artifact and nothing else.
- **Clean up empty parent directories.** After removing the skill file, the agent removes any directories that are now empty up to the environment root.
- **Not remove other files.** Sibling skills, user-created files, and unrelated content in the same tree are never touched.

## Evals

Planned evaluations that verify the agentic contracts above. Each eval is a scenario the agent must handle correctly.

| Eval | Contract | Description |
|------|----------|-------------|
| [install_idempotent](evals/install_idempotent.md) | Install | Installing a skill twice produces the same result -- no duplicates, no errors, identical output |
| [environment_path_resolution](evals/environment_path_resolution.md) | Install | Each target environment resolves to its correct native path, including Claude Code and Codex skill directories |
| [uninstall_cleanup](evals/uninstall_cleanup.md) | Uninstall | Uninstall removes the skill file and empty parent directories only |
| compose-skills fixtures | Check / Compose | Bundled fixtures cover malformed plans, ambiguous one-skill-vs-many splits, and an agent-doc workflow decomposition example |
