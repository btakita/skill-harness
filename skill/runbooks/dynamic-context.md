# Dynamic Context For Skills

Use a skill directory as the portable unit of context. `SKILL.md` should say when
the skill applies and which resource to load; detailed context belongs in sibling
files that `skill-harness install-dir` copies and `check-dir` audits.

## Resource Layout

- `SKILL.md`: trigger description, core rules, and a resource map.
- `runbooks/`: procedures loaded only for a specific operation.
- `references/`: schemas, domain rules, API notes, and service contracts.
- `scripts/`: deterministic pack generation, validation, or conversion.
- `assets/`: templates or files consumed as outputs, not prompt context.
- `okf/`: Open Knowledge Format concept bundles validated before install.

Do not duplicate a procedure in `SKILL.md` and a runbook. Keep one canonical owner
and link to it from the router entry.

## Install Policy

Use:

```bash
skill-harness install-dir <name> --source <skill-directory> --harness <target>
skill-harness check-dir <name> --source <skill-directory> --harness <target>
```

Use single-file `install` only for a skill with no companion files. If a skill
depends on dynamic context, `install-dir` is the default because it preserves the
resource graph across Claude, Codex, OpenCode, and generic layouts.

## Service And DB Guidance

For mutable context, put a small command behind the skill instead of embedding a
snapshot. The command should emit bounded markdown or JSON with source handles,
hashes, and expansion commands. SQLite works well for local context packs:

- `resource_index`: skill name, relative path, content hash, summary.
- `pack_cache`: query, budget, input hashes, generated pack, expiry.
- `validation_runs`: skill name, command, result, timestamp, artifact path.

The database caches generated context; committed files remain the policy source.
Invalidate cache rows whenever referenced resource hashes or tool versions change.
