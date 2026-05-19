# Ambiguous Skill Split

## Decision Boundary

The workflow is a documentation cleanup flow with one user-facing goal. Keep it as one skill unless the reference-audit step becomes reusable across unrelated repositories.

## Proposed Skills

- name: documentation-cleanup
  purpose: Audit stale docs, propose edits, and update the affected files.
  triggers: "clean up docs", "audit stale docs", or a direct documentation maintenance request.
  inputs: repository docs, current implementation files, and optional style guides.
  outputs: targeted documentation edits and a short verification note.
  resources: SKILL.md plus references/examples.md for before/after examples.
  invocation: implicit
  handoff: ordinary repo verification workflow.

## Resource Inventory

Use references for before/after examples. Do not split a deterministic checker until repeated repositories need the same parser.

## Invocation Policy

The skill is implicit because it only reads and edits local documentation.

## Validation Plan

Forward-test one prompt that should stay in documentation-cleanup, one prompt that should hand off to a publishing workflow, and one prompt that should remain ordinary code review.

## Recommendation

Keep one skill for now. Revisit a split if the reference-audit step becomes independently reusable.

