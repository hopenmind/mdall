<div align="center">
  <img src="assets/Logo.png" alt="MD -> ALL" width="360"/>
</div>

<br/>

<div align="center">
  <strong>Write your equations once. Export everywhere. Recover everything.</strong>
</div>

<br/>

> **MD -> ALL** is a self-contained scientific Markdown editor with full LaTeX/KaTeX rendering and lossless multi-format export. No runtime, no install prerequisites: download one executable, run it, everything works.

<br/>

## Download

Latest release, self-contained (binaries plus a bundled PDF engine, ready to run):

| Platform | Download |
|---|---|
| Windows x64 | [mdall-win-x64.zip](https://github.com/hopenmind/mdall/releases/latest/download/mdall-win-x64.zip) |
| Linux x64 | [mdall-linux-x64.zip](https://github.com/hopenmind/mdall/releases/latest/download/mdall-linux-x64.zip) |
| macOS (Apple Silicon) | [mdall-macos-arm64.zip](https://github.com/hopenmind/mdall/releases/latest/download/mdall-macos-arm64.zip) |

Just the MCP server (headless converter, no GUI, lighter download):

| Platform | Download |
|---|---|
| Windows x64 | [mdall-mcp-win-x64.zip](https://github.com/hopenmind/mdall/releases/latest/download/mdall-mcp-win-x64.zip) |
| Linux x64 | [mdall-mcp-linux-x64.zip](https://github.com/hopenmind/mdall/releases/latest/download/mdall-mcp-linux-x64.zip) |
| macOS (Apple Silicon) | [mdall-mcp-macos-arm64.zip](https://github.com/hopenmind/mdall/releases/latest/download/mdall-mcp-macos-arm64.zip) |

All versions and changelogs: [github.com/hopenmind/mdall/releases](https://github.com/hopenmind/mdall/releases)

<br/>

## Architecture

<div align="center">
  <img src="assets/readme/architecture.svg" alt="Architecture" width="100%"/>
</div>

<br/>

## Reversible DOCX: The Lossless Cycle

<div align="center">
  <img src="assets/readme/reversibility.svg" alt="Reversibility" width="100%"/>
</div>

The core innovation: **DOCX export is not destructive**. Every LaTeX equation is preserved in three independent redundant locations inside the file, so the original Markdown + LaTeX source can be recovered perfectly after any Word round-trip.

| Layer | Location | Survives |
|---|---|---|
| **Primary** | `md-to-all-source.xml` custom ZIP entry | Word open/save, annotation, track changes |
| **Secondary** | PNG `tEXt` ancillary chunk (`LaTeX: ...`) | Image extraction, copy-paste |
| **Tertiary** | SVG `<metadata>` CDATA block | SVG re-use, vector export |

**Workflow**: Researcher writes in MD -> ALL, exports DOCX, supervisor annotates in Word, researcher re-imports in MD -> ALL, original Markdown + all LaTeX equations recovered intact.

<br/>

## Export Formats

| Format | Quality | LaTeX rendering | Notes |
|---|---|---|---|
| **PDF** (Tier 1) | best | KaTeX pixel-perfect | Bundled ungoogled-chromium via CDP |
| **PDF** (Tier 2) | high | Typst, New Computer Modern Math | Pure Rust, zero system deps |
| **PDF** (Tier 3) | basic | Unicode approximation | genpdf fallback, always works |
| **HTML** | best | KaTeX, server-side rendered | Self-contained, embedded CSS, offline |
| **DOCX** | high | SVG/PNG equations | **Reversible**, re-importable to Markdown |
| **ODT** | high | SVG/PNG equations | LibreOffice compatible |
| **EPUB** | high | SVG equations | E-reader compatible |
| **LaTeX** | best | Native pass-through | `.tex` source, equation-preserving |
| **Typst** | best | Native conversion | Auto-converted LaTeX -> Typst math |
| **RTF** | basic | Unicode approximation | Word/legacy compatible |
| **TXT** | basic | Unicode approximation | Plain text, always readable |
| **SVG** | best | Vector equations | Per-equation, embeds LaTeX source |

<br/>

## LaTeX Support

MD -> ALL handles LaTeX in all its forms, as written by researchers in real scientific papers:

```latex
% Display math: all delimiters recognized
$$  \nabla^2 \phi = \frac{\rho}{\varepsilon_0}  $$

\[  \int_0^\infty e^{-x^2} dx = \frac{\sqrt{\pi}}{2}  \]

% Inline math
The energy $E = mc^2$ where $m$ is rest mass.

\( \hat{H}\psi = E\psi \)

% Environments
\begin{align}
  \dot{x} &= \sigma(y - x) \\
  \dot{y} &= x(\rho - z) - y
\end{align}

% Complex operators
\operatorname{softmax}(\mathbf{z})_i = \frac{e^{z_i}}{\sum_j e^{z_j}}

% Subscript notation
h_{\text{Center\_State}} = \tanh(W_{\text{fwd}} \cdot x_t)
```

**Normalization pipeline**: double-escaped LaTeX (`\\alpha`), markdown-escaped braces (`\{`), and mixed notation are all normalized automatically before rendering.

<br/>

## Editor

The editor offers **two synchronized panels**:

- **Source panel**: raw Markdown with LaTeX syntax, full editing
- **Render panel**: live preview with KaTeX-rendered equations, images resolved

Both panels are interoperable: edits in source reflect instantly in render. The toolbar wraps selected text (bold, italic, strikethrough, underline, super/subscript, code, alignment, inline and block equations).

### Roadmap: Full Bidirectional WYSIWYG

The next milestone turns MD -> ALL into a true scientific document editor on par with LibreOffice Writer, with these additional capabilities:

| Feature | Status |
|---|---|
| Click equation in render to edit LaTeX inline | Next |
| Type directly in rendered view, syncs to source | Next |
| Color picker, font size, highlight in render panel | Next |
| Table editor in rendered view | Planned |
| Image drag-and-drop with auto-reference | Planned |
| Comment/annotation layer (shared with DOCX) | Planned |
| Real-time collaborative editing | Future |

<br/>

## MCP Server (`mdall-mcp`)

`mdall-mcp` exposes the MD -> ALL conversion engine to any MCP client (an
automation host that speaks the [Model Context Protocol](https://modelcontextprotocol.io))
over stdio. It runs headless, fully offline, and shares the editor's exact
conversion core, including the lossless DOCX round-trip. It is a separate,
self-contained binary: you can use it on its own, without the editor.

### Install

Pick one:

- **Download** `mdall-mcp-<platform>.zip` from the [releases](https://github.com/hopenmind/mdall/releases/latest) and unzip it.
- **Build** from source: `cargo build --release -p mdall-mcp` -> `target/release/mdall-mcp(.exe)`.

No runtime is required: it is a single executable. Note its absolute path.

### Configure your MCP client

Point any MCP-compatible client at the binary. The server speaks MCP over stdio
(newline-delimited JSON-RPC 2.0), so the configuration is just the command:

```json
{
  "mcpServers": {
    "mdall": {
      "command": "C:/path/to/mdall-mcp.exe"
    }
  }
}
```

### Tools

| Tool | Arguments | Returns |
|---|---|---|
| `list_formats` | (none) | Every import (45) and export (18) format the engine supports. |
| `convert_file` | `{ input, output }` | Converts by file extension. DOCX export stays reversible. |
| `import_to_md` | `{ input }` | Any document returned as Markdown (LaTeX preserved). |
| `export_md` | `{ markdown, output, title?, author?, base_dir? }` | Writes Markdown to a target format; resolves relative images against `base_dir`. |
| `recover_source` | `{ input }` | Recovers the original Markdown + LaTeX from a DOCX produced by MD -> ALL. |

Paths are absolute. `convert_file` and `export_md` infer the target format from
the output extension (`.pdf`, `.docx`, `.html`, `.typ`, `.epub`, `.odt`, `.rtf`,
`.tex`, `.md`, ...). PDF uses the bundled engine when present and otherwise the
pure-Rust Typst tier, so it works with the standalone MCP binary too.

### The reversibility feature

`recover_source` is the differentiator: a DOCX exported by MD -> ALL embeds its
original Markdown + equation LaTeX in three redundant layers, so even after a
reviewer annotates it in Word, the exact editable source comes back.

```
author MD  --convert_file-->  paper.docx  --(annotated in Word)-->  paper.docx
                                                                        |
                              recover_source  <------------------------ /
                                    |
                              original Markdown + LaTeX, intact
```

### Protocol (manual test)

Transport is newline-delimited JSON-RPC 2.0 on stdin/stdout; the server speaks
MCP revision `2024-11-05`. You can drive it by hand, one JSON object per line:

```jsonc
{"jsonrpc":"2.0","id":1,"method":"initialize","params":{}}
{"jsonrpc":"2.0","id":2,"method":"tools/list"}
{"jsonrpc":"2.0","id":3,"method":"tools/call","params":{"name":"convert_file","arguments":{"input":"/abs/in.md","output":"/abs/out.pdf"}}}
{"jsonrpc":"2.0","id":4,"method":"tools/call","params":{"name":"recover_source","arguments":{"input":"/abs/out.docx"}}}
```

<br/>

## Zero External Dependencies

MD -> ALL is fully self-contained. The end user downloads **one file**, runs it, and everything works.

```
mdall-3.0.0-x64-installer.exe   (~179 MB)
|
+-- mdall.exe                    (34 MB, Rust binary)
|   +-- KaTeX engine    (duktape JS, compiled in)
|   +-- Typst 0.11      (math engine, compiled in)
|   +-- New CM Math     (OpenType MATH font, compiled in)
|   +-- All export logic (PDF/HTML/DOCX/EPUB/ODT/TeX/RTF/TXT)
|
+-- chromium/                        (342 MB, stripped headless build)
    +-- chrome.exe + minimal DLLs
        (ungoogled-chromium 148, headless-PDF-only strip)
```

- No VC++ Runtime required
- No .NET required
- No Node.js, no Python
- No Chrome/Edge/Firefox installation
- No internet access at runtime

<br/>

## Getting Started (Development)

```powershell
# 1. Clone
git clone https://github.com/hopenmind/mdall
cd mdall

# 2. Download bundled Chromium (one-time, ~400 MB)
.\scripts\setup-chromium.ps1

# 3. Run
cargo run

# 4. Build release binaries for every target (x64 + ARM64)
.\scripts\build-all.ps1

# 5. Build the self-contained installer
.\scripts\make-installer.ps1
```

<br/>

## Platform Support

| Target | Triple | Bundled PDF engine | Status |
|---|---|---|---|
| Windows x64 | `x86_64-pc-windows-msvc` | Chromium (bundled) | Supported, primary |
| Linux x64 | `x86_64-unknown-linux-gnu` | Chromium (bundled) | Supported |
| macOS arm64 | `aarch64-apple-darwin` | Chromium (bundled) | Supported |

Every target is built natively by CI (`.github/workflows/release.yml`) on
GitHub's own runners, so no local cross toolchain is needed: push a `vX.Y.Z` tag
and the bundles are produced for you. The GUI embeds a serif (New Computer
Modern) so it renders with no system font installed, and PDF export always falls
back to the pure-Rust Typst tier where a bundled engine is unavailable. The PDF
engine is selectable in-app under Options (Native or General converter).

Windows ARM64 is on hold: the KaTeX JS engine (duktape) does not compile on
`aarch64-pc-windows-msvc`, so that target awaits an alternative JS backend.

<br/>

## PDF Export: Three-Tier Cascade

```
Export PDF triggered
       |
       v
+---------------------------------------------+
| Tier 1: ungoogled-chromium CDP              |  <- Best quality
|   HTML (KaTeX pre-rendered) -> Chrome -> PDF|     Pixel-perfect math
|   Bundled binary, no system browser needed  |
+------------------+--------------------------+
         fails?    |
                   v
+---------------------------------------------+
| Tier 2: Typst pure-Rust                     |  <- Excellent quality
|   Markdown -> Typst source -> PDF in memory |     New Computer Modern Math
|   Zero system dependencies                  |
+------------------+--------------------------+
         fails?    |
                   v
+---------------------------------------------+
| Tier 3: genpdf fallback                     |  <- Always works
|   LaTeX -> Unicode approximation -> PDF     |     System fonts (Segoe/Arial)
+---------------------------------------------+
```

<br/>

## Technical Stack

| Component | Technology |
|---|---|
| GUI | egui 0.29 / eframe (immediate-mode, pure Rust) |
| Markdown | pulldown-cmark 0.12 |
| KaTeX (server-side) | `katex` crate + duktape JS engine |
| Math equations (preview) | Typst 0.11 + `typst-render` to PNG |
| PDF Tier 1 | `chromiumoxide` 0.5, CDP protocol |
| PDF Tier 2 | `typst-pdf` 0.11 |
| PDF Tier 3 | `rckive-genpdf` 0.4 |
| DOCX / ODT | `zip` crate, raw OpenDocument XML |
| EPUB | `epub-builder` 0.7 |
| DOCX reversibility | PNG tEXt CRC-32 / SVG `<metadata>` / DOCX custom ZIP entry |
| Cross-compilation | `cargo-zigbuild` + Zig linker |
| Installer | Rust self-extracting stub (payload appended to exe tail) |

<br/>

## License

(c) 2024-2025 Hope 'n Mind SASU - contact@hopenmind.com
All rights reserved. Research use permitted with attribution.

> *"Write your equations once. Export everywhere. Recover everything."*

<br/>

<div align="center">
  <sub>A research project by</sub>
  <br/><br/>
  <img src="assets/logo-hm.png" alt="Hope 'n Mind" width="150"/>
  <br/>
  <strong>Hope 'n Mind</strong>
</div>
