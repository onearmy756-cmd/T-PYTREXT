//! events.rs — hujenga events za agent-server (schema halisi) kwa Rust.
//!
//! Kila event ni JSON yenye `kind` (mkataba wa OpenHands). Frames za WS
//! (`/sockets/session/{id}`) zina envelope: sync, durable, transient,
//! item_started, delta, item_aborted, error.

use serde_json::{json, Value};

use super::state::{new_id, now_rfc3339};

/// MessageEvent — ujumbe wa user/assistant (llm_message + content parts).
pub fn message_event(role: &str, text: &str, source: &str, parent_id: Option<&str>) -> Value {
    message_event_with_id(&new_id(), role, text, source, parent_id)
}

/// MessageEvent yenye id maalum — LAZIMA ilingane na `item_id` ya `item_started`
/// ili Canvas ifunge slot ya mtiririko (`event.id === item_id`).
pub fn message_event_with_id(
    id: &str,
    role: &str,
    text: &str,
    source: &str,
    parent_id: Option<&str>,
) -> Value {
    json!({
        "id": id,
        "timestamp": now_rfc3339(),
        "source": source,
        "parent_id": parent_id,
        "llm_message": {
            "role": role,
            "content": [ { "cache_prompt": false, "type": "text", "text": text } ],
            "thinking_blocks": [],
        },
        "activated_skills": [],
        "extended_content": [],
        "kind": "MessageEvent",
    })
}

/// SystemPromptEvent — prompt ya mfumo (source agent).
pub fn system_prompt_event(text: &str) -> Value {
    json!({
        "id": new_id(),
        "timestamp": now_rfc3339(),
        "source": "agent",
        "system_prompt": { "cache_prompt": false, "type": "text", "text": text },
        "kind": "SystemPromptEvent",
    })
}

/// ConversationStateUpdateEvent — hali ya conversation (`execution_status`, n.k.).
pub fn state_update(key: &str, value: Value, parent_id: Option<&str>) -> Value {
    json!({
        "id": new_id(),
        "timestamp": now_rfc3339(),
        "source": "environment",
        "parent_id": parent_id,
        "kind": "ConversationStateUpdateEvent",
        "key": key,
        "value": value,
    })
}

/// Zana zinazotekelezwa kama amri ya shell kwenye Canvas (`ExecuteBashAction`).
fn is_bash_like(name: &str) -> bool {
    matches!(
        name,
        "code_execute"
            | "run_tests"
            | "build_run"
            | "build_targets"
            | "selfcheck"
            | "code_audit"
            | "code_audit_snippet"
            | "e2e_run"
    )
}

/// Amri inayoonyeshwa kwenye kadi ya terminal (maandishi yanayosomeka).
fn bash_command(name: &str, args: &Value) -> String {
    if let Some(c) = args.get("code").and_then(|v| v.as_str()) {
        return c.to_string();
    }
    match args.get("path").and_then(|v| v.as_str()) {
        Some(p) if !p.is_empty() => format!("{name} {p}"),
        _ => name.to_string(),
    }
}

/// ActionEvent ya zana — `kind` inalingana na Canvas (`FileEditorAction`,
/// `ExecuteBashAction` …) na hoja zake HALISI. Canvas hutambua kinds hizi na
/// kuchora kadi za zana (path/diff/terminal) badala ya maandishi ya kawaida,
/// hivyo kazi ya agent inaonekana kama kwenye OpenHands.
///
/// Hurudisha `(event, action_id, path)` — `action_id` huunganisha observation
/// na action yake (UI hutafuta `msg.id === observation.action_id`).
pub fn tool_action(
    tool_name: &str,
    args: &Value,
    parent_id: Option<&str>,
) -> (Value, String, Option<String>) {
    let arg_path = args
        .get("path")
        .and_then(|v| v.as_str())
        .filter(|p| !p.is_empty())
        .map(String::from);
    let action = match tool_name {
        "write_file" => json!({
            "kind": "FileEditorAction",
            "command": "create",
            "path": arg_path.clone().unwrap_or_default(),
            "file_text": args.get("content").and_then(|v| v.as_str()).unwrap_or(""),
            "old_str": Value::Null,
            "new_str": Value::Null,
            "insert_line": Value::Null,
            "view_range": Value::Null,
        }),
        "read_file" => json!({
            "kind": "FileEditorAction",
            "command": "view",
            "path": arg_path.clone().unwrap_or_default(),
            "file_text": Value::Null,
            "old_str": Value::Null,
            "new_str": Value::Null,
            "insert_line": Value::Null,
            "view_range": Value::Null,
        }),
        "list_files" => json!({ "kind": "GlobAction", "pattern": "*", "path": "" }),
        _ if is_bash_like(tool_name) => json!({
            "kind": "ExecuteBashAction",
            "command": bash_command(tool_name, args),
            "is_input": false,
            "timeout": Value::Null,
        }),
        _ => json!({ "kind": "MCPToolAction", "data": args }),
    };
    let id = new_id();
    let event = json!({
        "id": id,
        "timestamp": now_rfc3339(),
        "source": "agent",
        "parent_id": parent_id,
        "kind": "ActionEvent",
        "action": action,
        "tool_name": tool_name,
        "tool_call_id": new_id(),
        "llm_response_id": new_id(),
        "thought": [],
    });
    (event, id, arg_path)
}

/// ObservationEvent — matokeo ya kitendo, imeunganishwa na `action_id` ya
/// action iliyoiita (bila hiyo Canvas haioni uhusiano wa action→observation).
pub fn tool_observation(
    tool_name: &str,
    action_id: &str,
    path: Option<&str>,
    output: &str,
    parent_id: Option<&str>,
) -> Value {
    let text_part = json!([{ "cache_prompt": false, "type": "text", "text": output }]);
    let observation = match tool_name {
        "write_file" | "read_file" => json!({
            "kind": "FileEditorObservation",
            "command": if tool_name == "write_file" { "create" } else { "view" },
            "path": path,
            "output": output,
            "prev_exist": false,
            "old_content": Value::Null,
            "new_content": Value::Null,
            "error": Value::Null,
            "content": text_part,
        }),
        _ if is_bash_like(tool_name) => json!({
            "kind": "ExecuteBashObservation",
            "command": Value::Null,
            "exit_code": 0,
            "error": false,
            "timeout": false,
            "content": text_part,
            "metadata": {
                "exit_code": 0,
                "pid": 0,
                "username": Value::Null,
                "hostname": Value::Null,
                "working_dir": Value::Null,
                "py_interpreter_path": Value::Null,
                "prefix": "",
                "suffix": "",
            },
        }),
        _ => json!({
            "kind": "MCPToolObservation",
            "data": { "tool": tool_name, "output": output },
            "content": text_part,
        }),
    };
    json!({
        "id": new_id(),
        "timestamp": now_rfc3339(),
        "source": "environment",
        "parent_id": parent_id,
        "kind": "ObservationEvent",
        "observation": observation,
        "tool_name": tool_name,
        "tool_call_id": new_id(),
        "action_id": action_id,
    })
}

/// ActionEvent — kitendo cha agent (tool call).
pub fn action_event(tool_name: &str, args: Value, parent_id: Option<&str>) -> Value {
    tool_action(tool_name, &args, parent_id).0
}

/// ObservationEvent — matokeo ya kitendo.
pub fn observation_event(tool_name: &str, content: &str, parent_id: Option<&str>) -> Value {
    tool_observation(tool_name, &new_id(), None, content, parent_id)
}

/// ConversationErrorEvent yenye uainishaji kamili (auth, llm, n.k.).
pub fn error_event_classified(
    code: &str,
    detail: &str,
    classification: Value,
    parent_id: Option<&str>,
) -> Value {
    json!({
        "id": new_id(),
        "timestamp": now_rfc3339(),
        "source": "environment",
        "parent_id": parent_id,
        "code": code,
        "detail": detail,
        "classification": classification,
        "kind": "ConversationErrorEvent",
    })
}

// ── Frames za WS ─────────────────────────────────────────────────────────

/// `sync` — hutumwa mara moja kabla ya replay, hueleza wigo wa seq.
pub fn frame_sync(from_seq: Option<u64>, through_seq: Option<u64>) -> Value {
    json!({ "type": "sync", "from_seq": from_seq, "through_seq": through_seq })
}

/// `durable` — event iliyohifadhiwa; `seq` ni index yake kwenye log.
pub fn frame_durable(seq: u64, event: Value) -> Value {
    json!({ "type": "durable", "seq": seq, "event": event })
}

/// `transient` — event isiyohifadhiwa (haina seq).
pub fn frame_transient(event: Value) -> Value {
    json!({ "type": "transient", "event": event })
}

/// `item_started` — kipande cha stream kinaanza (slot kwa UI).
pub fn frame_item_started(item_id: &str, anchor_seq: Option<u64>) -> Value {
    json!({ "type": "item_started", "item_id": item_id, "attempt": 1, "anchor_seq": anchor_seq })
}

/// `delta` — kipande kimoja cha maandishi yanayotiririka.
pub fn frame_delta(item_id: &str, order: u64, content: &str) -> Value {
    json!({
        "type": "delta",
        "item_id": item_id,
        "attempt": 1,
        "order": order,
        "kind": "text",
        "content": content,
        "chunk_id": null,
        "choice_index": null,
    })
}

/// `item_aborted` — mtiririko uliisha bila event ya durable.
pub fn frame_item_aborted(item_id: &str, reason: &str) -> Value {
    json!({ "type": "item_aborted", "item_id": item_id, "attempt": 1, "reason": reason })
}

/// `error` — hitilafu ya socket.
pub fn frame_error(code: &str, detail: &str) -> Value {
    json!({ "type": "error", "code": code, "detail": detail })
}
