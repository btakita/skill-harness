# Compose Skills Functional Specification

## Purpose

`compose-skills` designs a small system of reusable skills before individual skills are implemented. It is a planning and audit skill, not a replacement for a skill-creation workflow.

## Inputs

- A workflow description, existing skill, or rough skill idea.
- Optional examples of user prompts and expected outputs.
- Optional existing skill paths for overlap analysis.

## Outputs

The skill emits a markdown architecture plan with:

- decision boundary for one skill versus many skills;
- proposed skill list with names, purposes, triggers, inputs, outputs, resources, invocation policy, and handoff points;
- resource inventory covering references, scripts, and assets;
- validation plan with positive, chained, and negative trigger tests;
- recommendation to create standalone skills, extend an existing skill, or keep the behavior as ordinary prompt guidance.

If the plan leads to implementation, it must name where `compose-skills` stops and where `skill-creator` begins. `compose-skills` owns architecture and validation strategy; `skill-creator` owns creating or updating specific skill files after approval.

## Rules

1. Prefer a single skill until splitting buys reuse, safety, clearer validation, or lower maintenance.
2. Split planning, deterministic tooling, risky side effects, and domain-specific references when they have different triggers or policies.
3. Keep `SKILL.md` concise and move detailed examples or domain rules into references.
4. Use deterministic commands for repeated checks or transformations.
5. Mark high-risk actions as explicit or user-only; do not hide deploys, sends, purchases, or publication behind implicit routing.
6. Hand off individual skill implementation to the relevant skill-creation workflow after the composition plan is validated.

## Required Plan Sections

Every composition plan must include:

- `Decision Boundary`
- `Proposed Skills`
- `Resource Inventory`
- `Invocation Policy`
- `Validation Plan`
- `Recommendation`

## Validation Contract

`skill-harness compose validate <plan.md>` must fail when required sections are absent, when a plan has no candidate skill entries, when the decision boundary lacks an explicit one-skill-vs-many rationale, or when skill creation is mentioned without a `skill-creator` handoff boundary. A candidate skill entry is any markdown list item or heading containing `name:` with a hyphen-case skill name.
