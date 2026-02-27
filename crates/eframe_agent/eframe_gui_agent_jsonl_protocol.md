# eframe GUI Agent JSONL Protocol

This document defines a minimal JSONL protocol for closed-loop GUI automation of egui/eframe apps.
It targets MVP workflows and kittest-based verification.

## Purpose
- Drive UI actions from a plain text script.
- Verify UI and (optionally) app state outcomes.
- Keep the protocol minimal and stable for early MVP iterations.

## Non-goals
- General purpose UI testing framework.
- Multi-client or networked test orchestration.
- Production-grade security or sandboxing.

## File format
- Encoding: UTF-8.
- One JSON object per line (JSONL).
- Blank lines are allowed and ignored.
- Comments are not allowed.

## Common fields
All records MAY include:
- `id` (string): stable identifier for logs and diffs.
- `label` (string): human-readable description.
- `ts` (string): timestamp for debugging (ISO-8601 or any string).

All records MUST include:
- `kind` (string): record type.

## Record types

### meta
Describes the script and schema version. The first non-empty record MUST be `meta`.

Required fields:
- `kind`: `"meta"`
- `schema_version`: integer

Optional fields:
- `app`: string
- `env`: object

### action
Drives UI interactions.

Required fields:
- `kind`: `"action"`
- `action`: `"click" | "focus" | "type_text" | "press_key" | "run_steps" | "sleep_ms"`

Optional fields (by action):
- `target`: `{ "by": "label" | "role" | "text" | "id", "value": string }`
- `text`: string (for `type_text`)
- `key`: string (for `press_key`, e.g. `"Enter"`, `"K"`)
- `modifiers`: array of strings (for `press_key`, e.g. `["ctrl", "shift"]`)
- `steps`: integer (for `run_steps`)
- `dt`: number, seconds per step (for `run_steps`)
- `ms`: integer (for `sleep_ms`)

### expect
Verifies UI or state outcomes.

Required fields:
- `kind`: `"expect"`
- `checks`: array of check objects

Optional fields:
- `within_ms`: integer (deadline for all checks)

Check object shapes:
- UI checks:
  - `{ "kind": "ui_exists", "target": { "by": "...", "value": "..." } }`
  - `{ "kind": "ui_text_contains", "value": string }`
- State checks (if the runner exposes a JSON state snapshot):
  - `{ "kind": "state_path_equals", "path": string, "value": any }`
  - `{ "kind": "state_path_contains", "path": string, "value": string }`

## Target selectors
- `by: "label"`: match accessible label text.
- `by: "role"`: match AccessKit role name (e.g. `"Button"`, `"TextInput"`).
- `by: "text"`: match visible text content.
- `by: "id"`: match the AccessKit `author_id` set by the app (e.g. `"prompt_input"`).

Notes:
- `label`/`text` queries map to AccessKit label/value fields produced by egui.
- `id` selectors require the app to set stable `author_id` values for widgets that need automation.

## State path syntax
When `state_path_*` checks are used, `path` MUST be a JSON Pointer (RFC 6901).
Example: `/messages/0/text`.

If the runner does not support state snapshots, it SHOULD fail fast when a state check is requested.

## Execution model
- Records are executed in file order.
- `action` records execute immediately.
- `expect` records poll until all checks pass or the deadline expires.
- If `within_ms` is omitted, the runner SHOULD apply a default deadline (runner-defined).
- The script fails on the first error unless the runner provides a "continue on error" mode.
- Runners may operate against a live app by injecting events and reading AccessKit updates, or against a headless harness.

## Error handling
The runner SHOULD report:
- the record index,
- the `id` (if present),
- and a short failure message.

## Extensibility
- Unknown fields SHOULD be ignored for forward compatibility.
- New record types MUST bump `schema_version`.
