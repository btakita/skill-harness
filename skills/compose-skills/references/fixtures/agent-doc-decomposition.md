# Agent-Doc Workflow Decomposition

## Decision Boundary

Agent-doc has one hot-path session loop, so keep the normal response cycle in one skill. Split compact exchange, route diagnostics, and commit repair into runbooks or helper skills because they have different risk controls and validation needs.

## Proposed Skills

- name: agent-doc-session
  purpose: Run the normal preflight, plan, response, finalize, and session-check cycle.
  triggers: "agent-doc <file>" or the harness-native agent-doc entrypoint.
  inputs: session document path, installed agent-doc CLI, and repository workspace.
  outputs: committed session response and local verification evidence.
  resources: SKILL.md plus runbooks for rare paths.
  invocation: explicit
  handoff: route, compact, or repair procedures when plan output requires them.

- name: agent-doc-route-diagnostics
  purpose: Analyze dispatch route proofs and fail closed when pane input acceptance is not enough.
  triggers: route diagnostics, tmux dispatch failures, or prompt-delivery uncertainty.
  inputs: route logs, live pane state, and harness-specific invocation rules.
  outputs: route diagnosis and next action.
  resources: references plus deterministic route-log fixtures.
  invocation: agent-only
  handoff: agent-doc-session after diagnostics produce a safe route.

## Resource Inventory

Keep the hot path in SKILL.md. Put route proof language, compact exchange, and repair flows in runbooks because they are detailed and only relevant under specific plan outputs.

## Invocation Policy

The session skill is explicit because it mutates the document and commits. Route diagnostics are agent-only because they are internal safety checks.

## Validation Plan

Test a normal agent-doc file invocation, a route diagnostics prompt that must not claim dispatch consumption from pane-input acceptance alone, and a generic markdown editing request that must not trigger the session loop.

## Recommendation

Keep agent-doc-session as the primary skill and split rare, safety-sensitive paths into runbooks or helper skills as their validation fixtures mature.
