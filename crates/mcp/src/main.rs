//! mdall-mcp - a headless Model Context Protocol server over the MD -> ALL
//! conversion core. It exposes document conversion across 40+ formats AND the
//! lossless DOCX -> Markdown LaTeX recovery (the reversibility differentiator)
//! as MCP tools, fully offline and dependency-light.
//!
//! Transport: newline-delimited JSON-RPC 2.0 on stdin/stdout (the MCP stdio
//! transport). One JSON object per line in, one per line out. Conversion is
//! synchronous, so there is no async runtime and no MCP SDK - just the core
//! engine plus `serde_json` for framing.

use mdall_core::convert;
use mdall_core::export::PdfMetadata;
use mdall_core::source_embed;
use serde_json::{json, Value};
use std::io::{self, BufRead, Write};
use std::path::Path;

/// MCP protocol revision this server speaks.
const PROTOCOL_VERSION: &str = "2024-11-05";

fn main() {
    let stdin = io::stdin();
    let stdout = io::stdout();
    let mut out = stdout.lock();

    for line in stdin.lock().lines() {
        let line = match line {
            Ok(l) => l,
            Err(_) => break,
        };
        if line.trim().is_empty() {
            continue;
        }
        let req: Value = match serde_json::from_str(&line) {
            Ok(v) => v,
            Err(_) => continue, // ignore unparseable input rather than crash
        };
        let id = req.get("id").cloned();
        let method = req.get("method").and_then(Value::as_str).unwrap_or("");
        if let Some(resp) = handle(method, req.get("params"), id) {
            let _ = writeln!(out, "{}", resp);
            let _ = out.flush();
        }
    }
}

/// Dispatch a JSON-RPC request. Returns `None` for notifications (no reply).
fn handle(method: &str, params: Option<&Value>, id: Option<Value>) -> Option<Value> {
    match method {
        "initialize" => Some(ok(
            id?,
            json!({
                "protocolVersion": PROTOCOL_VERSION,
                "capabilities": { "tools": {} },
                "serverInfo": { "name": "mdall-mcp", "version": env!("CARGO_PKG_VERSION") }
            }),
        )),
        // Lifecycle notifications carry no id and expect no response.
        "notifications/initialized" | "notifications/cancelled" => None,
        "ping" => Some(ok(id?, json!({}))),
        "tools/list" => Some(ok(id?, json!({ "tools": tool_defs() }))),
        "tools/call" => Some(handle_call(params, id?)),
        _ => id.map(|id| err(id, -32601, "method not found")),
    }
}

// ── Tool definitions ─────────────────────────────────────────────────────────

fn tool_defs() -> Value {
    json!([
        {
            "name": "list_formats",
            "description": "List every import and export format the conversion engine supports.",
            "inputSchema": { "type": "object", "properties": {}, "additionalProperties": false }
        },
        {
            "name": "convert_file",
            "description": "Convert a document from one format to another, inferring both formats from the file extensions. Fully offline. DOCX export is reversible: the original Markdown + LaTeX is embedded for lossless recovery.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "input":  { "type": "string", "description": "Absolute path to the source file (.md/.docx/.html/.tex/...)." },
                    "output": { "type": "string", "description": "Absolute path to write; the extension selects the target format (.pdf/.docx/.html/.typ/...)." }
                },
                "required": ["input", "output"],
                "additionalProperties": false
            }
        },
        {
            "name": "import_to_md",
            "description": "Import any supported document and return its Markdown representation (LaTeX equations preserved as $...$). Does not write a file.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "input": { "type": "string", "description": "Absolute path to the source file." }
                },
                "required": ["input"],
                "additionalProperties": false
            }
        },
        {
            "name": "export_md",
            "description": "Export inline Markdown to a file in the format implied by the output extension. Optional title/author metadata. Referenced figures are resolved relative to base_dir (default: the output folder).",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "markdown": { "type": "string", "description": "Markdown source to export." },
                    "output":   { "type": "string", "description": "Absolute output path; extension selects the format." },
                    "title":    { "type": "string", "description": "Optional document title." },
                    "author":   { "type": "string", "description": "Optional document author." },
                    "base_dir": { "type": "string", "description": "Optional folder to resolve relative image paths against (default: output folder)." }
                },
                "required": ["markdown", "output"],
                "additionalProperties": false
            }
        },
        {
            "name": "recover_source",
            "description": "Recover the ORIGINAL editable Markdown + LaTeX from a DOCX previously exported by MD -> ALL. This is the reversibility differentiator: a supervisor can annotate the DOCX in Word and the author recovers their exact source. Returns the recovered Markdown.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "input": { "type": "string", "description": "Absolute path to a .docx produced by MD -> ALL." }
                },
                "required": ["input"],
                "additionalProperties": false
            }
        }
    ])
}

// ── Tool dispatch ─────────────────────────────────────────────────────────────

fn handle_call(params: Option<&Value>, id: Value) -> Value {
    let params = match params {
        Some(p) => p,
        None => return err(id, -32602, "missing params"),
    };
    let name = params.get("name").and_then(Value::as_str).unwrap_or("");
    let args = params.get("arguments").cloned().unwrap_or_else(|| json!({}));

    let result: Result<String, String> = match name {
        "list_formats" => Ok(list_formats()),
        "convert_file" => call_convert_file(&args),
        "import_to_md" => call_import_to_md(&args),
        "export_md" => call_export_md(&args),
        "recover_source" => call_recover_source(&args),
        other => Err(format!("unknown tool '{}'", other)),
    };

    match result {
        Ok(text) => ok(id, tool_text(&text, false)),
        Err(msg) => ok(id, tool_text(&msg, true)),
    }
}

fn str_arg<'a>(args: &'a Value, key: &str) -> Result<&'a str, String> {
    args.get(key)
        .and_then(Value::as_str)
        .ok_or_else(|| format!("missing or non-string argument '{}'", key))
}

fn list_formats() -> String {
    format!(
        "Import formats ({}):\n  {}\n\nExport formats ({}):\n  {}",
        convert::supported_import_exts().len(),
        convert::supported_import_exts().join(" "),
        convert::supported_export_exts().len(),
        convert::supported_export_exts().join(" "),
    )
}

fn call_convert_file(args: &Value) -> Result<String, String> {
    let input = str_arg(args, "input")?;
    let output = str_arg(args, "output")?;
    convert::convert_file(Path::new(input), Path::new(output))?;
    Ok(format!("converted: {} -> {}", input, output))
}

fn call_import_to_md(args: &Value) -> Result<String, String> {
    let input = str_arg(args, "input")?;
    convert::import_to_md(Path::new(input))
}

fn call_export_md(args: &Value) -> Result<String, String> {
    let markdown = str_arg(args, "markdown")?;
    let output = str_arg(args, "output")?;
    let out_path = Path::new(output);
    let mut meta = PdfMetadata::default();
    if let Some(t) = args.get("title").and_then(Value::as_str) {
        meta.title = t.to_string();
    }
    if let Some(a) = args.get("author").and_then(Value::as_str) {
        meta.author = a.to_string();
    }
    // Resolve relative figures against base_dir, else the output's own folder.
    let base = args
        .get("base_dir")
        .and_then(Value::as_str)
        .map(|s| Path::new(s).to_path_buf())
        .or_else(|| out_path.parent().map(Path::to_path_buf));
    convert::export_md(markdown, out_path, &meta, base.as_deref())?;
    Ok(format!("exported markdown -> {}", output))
}

fn call_recover_source(args: &Value) -> Result<String, String> {
    let input = str_arg(args, "input")?;
    source_embed::import_docx_source(Path::new(input))
}

// ── JSON-RPC helpers ──────────────────────────────────────────────────────────

fn ok(id: Value, result: Value) -> Value {
    json!({ "jsonrpc": "2.0", "id": id, "result": result })
}

fn err(id: Value, code: i64, message: &str) -> Value {
    json!({ "jsonrpc": "2.0", "id": id, "error": { "code": code, "message": message } })
}

/// Wrap text as an MCP tool result (`content` array with one text block).
fn tool_text(text: &str, is_error: bool) -> Value {
    json!({
        "content": [ { "type": "text", "text": text } ],
        "isError": is_error
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn initialize_and_tools_list() {
        let init = handle("initialize", Some(&json!({})), Some(json!(1))).unwrap();
        assert_eq!(init["result"]["protocolVersion"], PROTOCOL_VERSION);
        assert_eq!(init["result"]["serverInfo"]["name"], "mdall-mcp");
        let list = handle("tools/list", None, Some(json!(2))).unwrap();
        assert_eq!(list["result"]["tools"].as_array().unwrap().len(), 5);
    }

    #[test]
    fn initialized_notification_gets_no_reply() {
        assert!(handle("notifications/initialized", None, None).is_none());
    }

    #[test]
    fn convert_then_recover_round_trips_latex_via_tools() {
        // The reversibility differentiator, exercised through the MCP tool layer.
        let dir = std::env::temp_dir().join(format!("mdall_mcp_test_{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let md = dir.join("d.md");
        std::fs::write(&md, "# T\n\nInline $E=mc^2$ and\n\n$$\\int_0^1 x\\,dx = \\frac{1}{2}$$\n").unwrap();
        let docx = dir.join("d.docx");
        call_convert_file(&json!({ "input": md.to_str().unwrap(), "output": docx.to_str().unwrap() }))
            .expect("convert_file failed");
        let recovered = call_recover_source(&json!({ "input": docx.to_str().unwrap() }))
            .expect("recover_source failed");
        assert!(recovered.contains("$E=mc^2$"), "inline LaTeX not recovered: {recovered:?}");
        assert!(recovered.contains("\\frac{1}{2}"), "display LaTeX not recovered: {recovered:?}");
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn unknown_tool_is_an_error_result_not_a_crash() {
        let r = handle_call(Some(&json!({ "name": "nope", "arguments": {} })), json!(9));
        assert_eq!(r["result"]["isError"], true);
    }

    #[test]
    fn list_formats_reports_import_and_export() {
        let t = list_formats();
        assert!(t.contains("Import formats"));
        assert!(t.contains("Export formats"));
        assert!(t.contains("docx") && t.contains("pdf"));
    }
}
