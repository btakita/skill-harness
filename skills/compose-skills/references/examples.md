# Compose Skills Examples

## Content Workflow

Split a broad content workflow only when the steps are reusable. A `research-topic` skill can own source collection and notes, while `draft-social-post` can own channel-specific writing. Keep them together if the workflow always has one target and no reusable intermediate artifact.

## Oversized Workflow Handoff

For a broad prompt that asks for research, writing, packaging, and validation, produce the architecture first and hand off only the approved file-generation step to `skill-creator`. See `references/fixtures/oversized-workflow-handoff.md` for a complete source prompt, decomposed plan, validation matrix, and handoff boundary.

## Agent-Doc Workflow

Do not create a single giant skill for all session behavior if routing diagnostics, commit closeout, compact exchange, and repair flows each have distinct risk controls. Keep the hot path short and move rare cases into runbooks.

## Dynamic Context

When a proposed AGENTS or SKILL entry names a behavior, pair it with the file or
command that supplies the details at use time. Examples: `runbooks/commit.md` for
closeout, `references/schema.md` for domain fields, or `context pack --query ...`
for generated state.

## Browser Automation

Separate browser-control setup from site-specific operations when the same automation harness supports multiple sites. Keep site credentials, selectors, and safety policy in the site-specific skill.
