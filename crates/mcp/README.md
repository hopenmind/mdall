<div align="center">
  <img src="../../assets/Logo.png" alt="MD -> ALL" width="200"/>
</div>

<br/>

# mdall-mcp

A headless [Model Context Protocol](https://modelcontextprotocol.io) server over
the **MD -> ALL** conversion core. Any MCP client can drive it to convert
documents across 40+ formats and, uniquely, **recover the original editable
Markdown + LaTeX** from a DOCX that MD -> ALL produced, even after a reviewer
annotated it in Word.

Fully offline, pure Rust, no external runtime tools (no pandoc / LibreOffice /
Node). Transport is newline-delimited JSON-RPC 2.0 over stdio.

## Build

```
cargo build --release -p mdall-mcp
# binary: target/release/mdall-mcp(.exe)
```

## Tools

| Tool | Purpose |
|------|---------|
| `list_formats` | List supported import (45) and export (18) formats. |
| `convert_file` | `{input, output}` : convert by file extension. DOCX export stays reversible. |
| `import_to_md` | `{input}` : return any document as Markdown (LaTeX preserved). |
| `export_md` | `{markdown, output, title?, author?, base_dir?}` : write Markdown to a target format. |
| `recover_source` | `{input}` : recover the ORIGINAL Markdown + LaTeX from a MD -> ALL DOCX. |

## The scientific round-trip

```
author MD  --convert_file-->  paper.docx  --(reviewer annotates in Word)-->  paper.docx
                                                                                  |
                              recover_source  <-------------------------------- /
                                    |
                              original MD + LaTeX, intact
```

## Configure in an MCP client

Add the server to your MCP client's server configuration:

```json
{
  "mcpServers": {
    "mdall": {
      "command": "C:/path/to/target/release/mdall-mcp.exe"
    }
  }
}
```

The server reads JSON-RPC requests on stdin and writes responses on stdout, one
JSON object per line. It speaks MCP protocol revision `2024-11-05`.
