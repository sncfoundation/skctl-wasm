//! skctl-wasm — the skctl CLI compiled to WebAssembly.
//!
//! This crate exposes small, pure, side-effect-free helpers over the
//! Sheeternetes Apps Script apiserver protocol:
//!
//!   GET  $WEBAPP_URL?token=$TOKEN&kind=pods|nodes|deployments|events  -> {items:[...]}
//!   POST $WEBAPP_URL  {token, action, ...}
//!
//! The functions here only *build* URLs / request bodies and *render*
//! responses. The actual network I/O (`fetch`) is done by the host page
//! (see `index.html`). Keeping I/O in JS makes the wasm module trivially
//! testable and avoids pulling `web-sys` Fetch bindings into the crate.

use serde_json::{json, Value};
use wasm_bindgen::prelude::*;

/// Percent-encode a query-string component (RFC 3986 unreserved set is left
/// as-is, everything else is `%XX`).
fn encode(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for b in s.bytes() {
        match b {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                out.push(b as char)
            }
            _ => out.push_str(&format!("%{:02X}", b)),
        }
    }
    out
}

/// Build the apiserver GET URL for a `kind` (pods|nodes|deployments|events).
///
/// ```ignore
/// get_url("https://script.google.com/macros/s/AAA/exec", "secret", "deployments")
/// // => "https://script.google.com/macros/s/AAA/exec?token=secret&kind=deployments"
/// ```
#[wasm_bindgen]
pub fn get_url(base: &str, token: &str, kind: &str) -> String {
    let base = base.trim_end_matches('/');
    format!(
        "{}?token={}&kind={}",
        base,
        encode(token),
        encode(kind)
    )
}

/// Build the POST body for `skctl apply`.
///
/// `deployments_json` may be either a manifest object `{"deployments":[...]}`
/// (as stored in the lab manifests) or a bare array `[...]`. Either way the
/// resulting body is `{"token","action":"apply","deployments":[...]}`.
#[wasm_bindgen]
pub fn apply_body(token: &str, deployments_json: &str) -> Result<String, JsError> {
    build_apply_body(token, deployments_json).map_err(|e| JsError::new(&e))
}

/// Host-side core of [`apply_body`]: `JsError` can only be constructed on a
/// wasm target, so the logic returns a plain `String` error and stays testable
/// with `cargo test`.
fn build_apply_body(token: &str, deployments_json: &str) -> Result<String, String> {
    let parsed: Value = serde_json::from_str(deployments_json)
        .map_err(|e| format!("invalid manifest JSON: {}", e))?;

    let deployments = match parsed.get("deployments") {
        Some(d) => d.clone(),
        None => parsed, // allow a bare array or a single object
    };

    let body = json!({
        "token": token,
        "action": "apply",
        "deployments": deployments,
    });
    Ok(body.to_string())
}

/// Build the POST body for `skctl scale <name> <replicas>`.
#[wasm_bindgen]
pub fn scale_body(token: &str, name: &str, replicas: i32) -> String {
    json!({
        "token": token,
        "action": "scale",
        "name": name,
        "replicas": replicas,
    })
    .to_string()
}

/// Build the POST body for `skctl delete <name>`.
#[wasm_bindgen]
pub fn delete_body(token: &str, name: &str) -> String {
    json!({
        "token": token,
        "action": "delete",
        "name": name,
    })
    .to_string()
}

/// Render a `{items:[...]}` apiserver response into a fixed-width text table.
///
/// `kind` selects the column set (defaults to a deployments view for unknown
/// kinds). Returns a human-readable table, or an `error: ...` line if the
/// input is not the expected shape — it never panics, so it is safe to call
/// directly on a raw fetch body.
#[wasm_bindgen]
pub fn render_table(items_json: &str, kind: &str) -> String {
    let parsed: Value = match serde_json::from_str(items_json) {
        Ok(v) => v,
        Err(e) => return format!("error: invalid JSON: {}", e),
    };

    let items = match parsed.get("items").and_then(Value::as_array) {
        Some(a) => a,
        None => return "error: response has no \"items\" array".to_string(),
    };

    let (headers, keys): (&[&str], &[&str]) = match kind {
        "pods" => (
            &["NAME", "DEPLOYMENT", "NODE", "PHASE", "CONTAINER"],
            &["name", "deployment", "node", "phase", "container_id"],
        ),
        "nodes" => (
            &["NAME", "IP", "CPU", "CPU_USED", "MEM", "STATUS", "HEARTBEAT"],
            &[
                "name",
                "ip",
                "cpu_total",
                "cpu_used",
                "mem_total",
                "status",
                "last_heartbeat",
            ],
        ),
        "events" => (
            &["TS", "KIND", "OBJECT", "MESSAGE"],
            &["ts", "kind", "object", "message"],
        ),
        // deployments (and anything else) fall through to the deployment view
        _ => (
            &["NAME", "IMAGE", "REPLICAS", "CPU", "MEM"],
            &["name", "image", "replicas", "cpu_req", "mem_req"],
        ),
    };

    if items.is_empty() {
        return format!("No {} found.", if kind.is_empty() { "resources" } else { kind });
    }

    let rows: Vec<Vec<String>> = items
        .iter()
        .map(|it| keys.iter().map(|k| cell(it, k)).collect())
        .collect();

    // Column widths = max of header and any cell in that column.
    let mut widths: Vec<usize> = headers.iter().map(|h| h.chars().count()).collect();
    for row in &rows {
        for (i, c) in row.iter().enumerate() {
            let w = c.chars().count();
            if w > widths[i] {
                widths[i] = w;
            }
        }
    }

    let fmt_row = |cols: &[String]| -> String {
        cols.iter()
            .enumerate()
            .map(|(i, c)| {
                let pad = widths[i].saturating_sub(c.chars().count());
                format!("{}{}", c, " ".repeat(pad))
            })
            .collect::<Vec<_>>()
            .join("  ")
            .trim_end()
            .to_string()
    };

    let mut out = String::new();
    let header_row: Vec<String> = headers.iter().map(|s| s.to_string()).collect();
    out.push_str(&fmt_row(&header_row));
    for row in &rows {
        out.push('\n');
        out.push_str(&fmt_row(row));
    }
    out
}

/// Extract a single cell value as display text. Strings are shown verbatim,
/// container ids are truncated to 12 chars (matching the bash skctl), missing
/// or null values become `-`, and other scalars are stringified.
fn cell(v: &Value, key: &str) -> String {
    match v.get(key) {
        Some(Value::String(s)) if key == "container_id" => {
            s.chars().take(12).collect()
        }
        Some(Value::String(s)) => s.clone(),
        None | Some(Value::Null) => "-".to_string(),
        Some(Value::Number(n)) => n.to_string(),
        Some(Value::Bool(b)) => b.to_string(),
        Some(other) => other.to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::Value;

    #[test]
    fn builds_get_url_and_encodes_token() {
        let u = get_url("https://x/exec/", "a b&c", "pods");
        assert_eq!(u, "https://x/exec?token=a%20b%26c&kind=pods");
    }

    #[test]
    fn apply_body_unwraps_manifest_object() {
        let body = apply_body("t", r#"{"deployments":[{"name":"web"}]}"#).unwrap();
        let v: Value = serde_json::from_str(&body).unwrap();
        assert_eq!(v["action"], "apply");
        assert_eq!(v["token"], "t");
        assert_eq!(v["deployments"][0]["name"], "web");
    }

    #[test]
    fn apply_body_accepts_bare_array() {
        let body = apply_body("t", r#"[{"name":"web"}]"#).unwrap();
        let v: Value = serde_json::from_str(&body).unwrap();
        assert_eq!(v["deployments"][0]["name"], "web");
    }

    #[test]
    fn apply_body_rejects_garbage() {
        assert!(build_apply_body("t", "not json").is_err());
    }

    #[test]
    fn scale_and_delete_bodies() {
        let s: Value = serde_json::from_str(&scale_body("t", "web", 3)).unwrap();
        assert_eq!(s["action"], "scale");
        assert_eq!(s["replicas"], 3);
        let d: Value = serde_json::from_str(&delete_body("t", "web")).unwrap();
        assert_eq!(d["action"], "delete");
        assert_eq!(d["name"], "web");
    }

    #[test]
    fn renders_deployments_table() {
        let json = r#"{"items":[
            {"name":"web","image":"nginx","replicas":2,"cpu_req":"100m","mem_req":"64Mi"},
            {"name":"api","image":"go","replicas":1,"cpu_req":"250m","mem_req":"128Mi"}
        ]}"#;
        let t = render_table(json, "deployments");
        let first = t.lines().next().unwrap();
        assert!(first.starts_with("NAME"));
        assert!(first.contains("IMAGE"));
        assert_eq!(t.lines().count(), 3);
        assert!(t.contains("web"));
        assert!(t.contains("128Mi"));
    }

    #[test]
    fn render_handles_bad_input_without_panic() {
        assert!(render_table("oops", "pods").starts_with("error:"));
        assert!(render_table("{}", "pods").starts_with("error:"));
        assert_eq!(render_table(r#"{"items":[]}"#, "pods"), "No pods found.");
    }
}
