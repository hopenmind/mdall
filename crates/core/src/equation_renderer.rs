// Equation rasterizer - LaTeX → Typst math → PNG via typst-render.
// Uses Typst's embedded fonts (New Computer Modern Math) for correct
// OpenType MATH rendering - no system fonts required.

use comemo::Prehashed;
use typst::diag::{FileError, FileResult};
use typst::eval::Tracer;
use typst::foundations::{Bytes, Datetime};
use typst::syntax::{FileId, Source};
use typst::text::{Font, FontBook};
use typst::visualize::Color;
use typst::{Library, World};

/// Render a LaTeX block equation to RGBA PNG bytes.
/// Returns `(png_bytes, None)` on success, `(None, Some(error))` on failure.
pub fn render_equation_png(latex: &str, scale: f32) -> (Option<Vec<u8>>, Option<String>) {
    let typst_math = crate::export_typst::latex_to_typst_math(latex);

    // Display math in Typst: `$ math $` (spaces = block/display mode).
    // Auto-sized page so the image fits the equation exactly.
    let source_str = format!(
        "#set page(width: auto, height: auto, margin: (x: 10pt, y: 8pt))\n\
         #set text(size: 13pt)\n\
         $ {} $\n",
        typst_math
    );

    let world = match EquationWorld::new(&source_str) {
        Some(w) => w,
        None => return (None, Some("Font loading failed".into())),
    };

    let mut tracer = Tracer::new();
    let doc = match typst::compile(&world, &mut tracer) {
        Ok(d) => d,
        Err(errs) => {
            let msgs: Vec<String> = errs.iter().map(|e| e.message.to_string()).collect();
            return (None, Some(format!("Typst: {}", msgs.join("; "))));
        }
    };

    let page = match doc.pages.first() {
        Some(p) => p,
        None => return (None, Some("No pages produced".into())),
    };

    let pixmap = typst_render::render(&page.frame, scale, Color::WHITE);
    match pixmap.encode_png() {
        Ok(bytes) => (Some(bytes), None),
        Err(e) => (None, Some(format!("PNG encode: {}", e))),
    }
}

// ── Minimal Typst world - embedded fonts only ─────────────────────────────

struct EquationWorld {
    library: Prehashed<Library>,
    book: Prehashed<FontBook>,
    fonts: Vec<Font>,
    source: Source,
}

impl EquationWorld {
    fn new(source_str: &str) -> Option<Self> {
        let mut book = FontBook::new();
        let mut fonts: Vec<Font> = Vec::new();

        // Load Typst's bundled fonts (includes New Computer Modern Math -
        // the OpenType MATH table font required for correct math rendering).
        for data in typst_assets::fonts() {
            let bytes = Bytes::from_static(data);
            for face_idx in 0u32.. {
                match Font::new(bytes.clone(), face_idx) {
                    Some(f) => {
                        book.push(f.info().clone());
                        fonts.push(f);
                    }
                    None => break,
                }
            }
        }

        if fonts.is_empty() {
            return None;
        }

        let source = Source::detached(source_str.to_string());

        Some(Self {
            library: Prehashed::new(Library::builder().build()),
            book: Prehashed::new(book),
            fonts,
            source,
        })
    }
}

impl World for EquationWorld {
    fn library(&self) -> &Prehashed<Library> { &self.library }
    fn book(&self) -> &Prehashed<FontBook> { &self.book }
    fn main(&self) -> Source { self.source.clone() }
    fn source(&self, id: FileId) -> FileResult<Source> {
        Err(FileError::NotFound(id.vpath().as_rootless_path().to_path_buf()))
    }
    fn file(&self, id: FileId) -> FileResult<Bytes> {
        Err(FileError::NotFound(id.vpath().as_rootless_path().to_path_buf()))
    }
    fn font(&self, index: usize) -> Option<Font> { self.fonts.get(index).cloned() }
    fn today(&self, _offset: Option<i64>) -> Option<Datetime> { None }
}

/// Render a LaTeX equation to SVG via Typst.
/// Returns the SVG string on success, None on failure.
pub fn render_equation_svg(latex: &str) -> Option<String> {
    let typst_math = crate::export_typst::latex_to_typst_math(latex);
    let source_str = format!(
        "#set page(width: auto, height: auto, margin: (x: 10pt, y: 8pt))\n\
         #set text(size: 13pt)\n\
         $ {} $\n",
        typst_math
    );
    let world = EquationWorld::new(&source_str)?;
    let mut tracer = Tracer::new();
    let doc = typst::compile(&world, &mut tracer).ok()?;
    let page = doc.pages.first()?;
    Some(typst_svg::svg(&page.frame))
}
