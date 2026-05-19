# Compose Skills Examples

## Content Workflow

Split a broad content workflow only when the steps are reusable. A `research-topic` skill can own source collection and notes, while `draft-social-post` can own channel-specific writing. Keep them together if the workflow always has one target and no reusable intermediate artifact.

## Agent-Doc Workflow

Do not create a single giant skill for all session behavior if routing diagnostics, commit closeout, compact exchange, and repair flows each have distinct risk controls. Keep the hot path short and move rare cases into runbooks.

## Browser Automation

Separate browser-control setup from site-specific operations when the same automation harness supports multiple sites. Keep site credentials, selectors, and safety policy in the site-specific skill.
