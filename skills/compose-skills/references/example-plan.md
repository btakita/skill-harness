# Skill Architecture Plan

## Decision Boundary

The workflow is larger than one skill because planning reusable skill boundaries and implementing each concrete skill have different failure modes and validation needs.

## Proposed Skills

- name: compose-skills
  purpose: Design a small system of cooperating skills before implementation.
  triggers: Requests to split an oversized workflow, audit skill composability, or decide whether one skill should become several.
  inputs: Workflow description, existing skill paths, and expected user prompts.
  outputs: Markdown architecture plan with boundaries, resources, invocation policy, and validation plan.
  resources: `SKILL.md`, `SPEC.md`, and `references/`.
  invocation: implicit.
  handoff: skill creation workflow.

- name: skill-creator
  purpose: Implement one approved skill from an architecture plan.
  triggers: Requests to create or update a specific skill.
  inputs: Approved skill scope, target directory, references, scripts, and metadata.
  outputs: Skill directory and validation evidence.
  resources: `SKILL.md`, templates, and validation scripts.
  invocation: implicit.
  handoff: ordinary code review and testing.

## Resource Inventory

`compose-skills` needs concise examples and deterministic plan validation. `skill-creator` owns concrete file templates and packaging checks.

## Invocation Policy

`compose-skills` is implicit because it is a low-risk planning skill. Any publishing, deploy, send, purchase, or remote mutation skill should be explicit or user-only.

## Validation Plan

Forward-test a prompt that should use only `compose-skills`, a prompt that should use `compose-skills` then `skill-creator`, and a prompt that should stay ordinary guidance.

## Recommendation

Keep `compose-skills` as a standalone planning skill and hand off implementation to `skill-creator` after the plan validates.
