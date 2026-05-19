# Oversized Workflow Handoff

## Source Prompt

Create a reusable skill system for launching a new product vertical. It should research customers, collect competitor examples, draft launch copy, create the final skill directories, and validate each skill.

## Decision Boundary

Keep one skill only for a narrow launch-writing request. This workflow should split into several skills because research, copywriting, and concrete skill implementation have different inputs, outputs, validation checks, and safety boundaries.

## Proposed Skills

- name: customer-research
  purpose: Collect audience notes, competitor examples, and open questions for a product vertical.
  triggers: Requests to research customers, market position, or competitor language before writing.
  inputs: Product brief, target audience, source URLs, and existing notes.
  outputs: Research brief with cited source notes and unanswered assumptions.
  resources: references/source-quality.md and examples/research-brief.md.
  invocation: implicit for research-only planning.
  handoff: launch-copy after the research brief is accepted.

- name: launch-copy
  purpose: Draft channel-specific launch copy from an approved research brief.
  triggers: Requests to write landing copy, email, social posts, or announcement copy.
  inputs: Approved research brief, channel list, voice guide, and constraints.
  outputs: Copy drafts plus validation notes for unsupported claims.
  resources: references/channel-patterns.md and examples/copy-set.md.
  invocation: implicit for local draft generation.
  handoff: skill-creator only after the user asks to package this workflow as installable skills.

- name: skill-creator
  purpose: Implement one approved skill directory from the architecture plan.
  triggers: Requests to create or update a specific skill after plan approval.
  inputs: Approved scope, target path, references, templates, and validation commands.
  outputs: Skill directory, copied resources, and validation evidence.
  resources: SKILL.md templates and packaging checks.
  invocation: explicit when files will be created or updated.
  handoff: ordinary code review and test verification after each skill is generated.

## Resource Inventory

Store source-quality rules in `customer-research/references/`, channel examples in `launch-copy/references/`, and file templates in `skill-creator/assets/`. Do not put customer research examples inside `skill-creator`; it should only package already-approved skill content.

## Invocation Policy

Research and copy skills can be implicit because they produce local planning or draft artifacts. `skill-creator` is explicit because it creates or edits files. The boundary is that `compose-skills` stops after the architecture plan and validation strategy; `skill-creator` begins only when the user approves implementation of a specific skill.

## Validation Plan

Forward-test an oversized prompt that should use `compose-skills` first, a research-only prompt that should use `customer-research`, a copy-only prompt that should use `launch-copy`, and a packaging prompt that should hand off to `skill-creator`. Negative-test a generic marketing critique request that should remain ordinary guidance.

## Recommendation

Split the oversized launch workflow into `customer-research`, `launch-copy`, and `skill-creator`. Use `compose-skills` for the architecture decision, then hand off to `skill-creator` only after the user approves a concrete skill implementation target.
