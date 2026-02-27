---
name: eframe-gui-test-skills
description: Guide for closed-loop egui verification using JSONL action/expect scripts.
---

# eframe gui test skills

## When to use
- You want an automated dev loop while building egui UIs: edit code -> build/run -> verify.

## Quick start
1. Use eframe_agent in your app, and enable the mcp_sse feature at startup to launch the MCP server for communication.
2. The user must configure MCP manually; once configured, you can detect and use the tools it exposes.
3. Call functions.list_mcp_resources to get mcp tools(for codex).
4. Run your closed-loop tests using JSONL (syntax below).
5. Iterate until the behavior matches the design.

## Closed-loop dev workflow
1. Edit your egui UI code.
2. Rebuild and run the app, then test using the MCP-provided tools.
3. Iterate until the behavior matches the design.

## MCP tool surface (live UI injection)
If the app exposes MCP tools for UI automation, the JSONL runner can be driven externally without custom code.

Typical MCP tools:
- `ui_click`: click a target (`label`/`text`/`role`/`id`).
- `ui_focus`: focus a target.
- `ui_type_text`: type text into a target.
- `ui_press_key`: press a key with optional modifiers.
- `ui_query`: query for UI existence or visible text.
- `ui_state_snapshot`: return a JSON state snapshot when supported.
- `run_jsonl`: run a full JSONL script and return a verdict.

Notes:
- Live UI injection routes actions into `raw_input_hook` by enqueueing AccessKit action requests and text/key events.
- AccessKit must be enabled in the app for UI querying to work reliably.

## JSONL format definition
JSONL is newline-delimited JSON. Each non-empty line is a standalone JSON object.

### File rules
- Encoding: UTF-8.
- One JSON object per line. No trailing commas.
- Blank lines are allowed and ignored.
- Comments are not allowed.

### Common fields
- `kind` (string, required): record type.
- `id` (string, optional): stable identifier for logs and diffs.
- `ts` (string, optional): timestamp for debugging (ISO-8601 or any string).
- `label` (string, optional): human-readable description.

### Record types

#### `meta`
Describes the script and schema version.

Required fields:
- `kind`: `"meta"`
- `schema_version`: integer

Optional fields:
- `app`: string
- `env`: object

#### `action`
Drives UI interactions.

Required fields:
- `kind`: `"action"`
- `action`: `"click" | "focus" | "type_text" | "press_key" | "run_steps" | "sleep_ms"`

Optional fields (by action):
- `target`: `{ "by": "label" | "role" | "text" | "id", "value": string }`
- `text`: string (for `type_text`)
- `key`: string (for `press_key`, e.g. "Enter", "K")
- `modifiers`: array of strings (e.g. `["ctrl", "shift"]`)
- `steps`: integer (for `run_steps`)
- `dt`: number (seconds per step, for `run_steps`)
- `ms`: integer (for `sleep_ms`)

#### `expect`
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
- State checks (if your runner exposes a JSON state snapshot):
  - `{ "kind": "state_path_equals", "path": string, "value": any }`
  - `{ "kind": "state_path_contains", "path": string, "value": string }`

### Target selectors
- `by: "label"`: match accessible label text.
- `by: "role"`: match accesskit role name.
- `by: "text"`: match visible text content.
- `by: "id"`: match the AccessKit `author_id` set by the app (e.g. `"prompt_input"`).

Notes:
- `label`/`text` queries map to AccessKit label/value fields produced by egui.
- `id` selectors require stable `author_id` values in the app.

### Example JSONL
```json
{"kind":"meta","schema_version":1,"app":"agent_demo"}
{"kind":"action","action":"click","target":{"by":"label","value":"Command Palette (Cmd+K)"}}
{"kind":"action","action":"focus","target":{"by":"role","value":"TextInput"}}
{"kind":"action","action":"type_text","target":{"by":"role","value":"TextInput"},"text":"hello from ui"}
{"kind":"action","action":"click","target":{"by":"label","value":"Send"}}
{"kind":"expect","within_ms":3000,"checks":[{"kind":"ui_text_contains","value":"echo: hello from ui"}]}
```
