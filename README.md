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
| Windows ARM64 | `aarch64-pc-windows-msvc` | Typst (pure-Rust) | Supported |
| Linux x64 | `x86_64-unknown-linux-gnu` | Chromium (bundled) | Supported |
| macOS arm64 | `aarch64-apple-darwin` | Chromium (bundled) | Supported |

Every target is built natively by CI (`.github/workflows/release.yml`) on
GitHub's own runners, so no local cross toolchain is needed: push a `vX.Y.Z` tag
and the bundles are produced for you. The GUI embeds a serif (New Computer
Modern) so it renders with no system font installed, and PDF export always falls
back to the pure-Rust Typst tier where a bundled engine is unavailable. The PDF
engine is selectable in-app under Options (Native or General converter).

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
