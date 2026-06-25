//! `mdall-core` - le noyau de MD -> ALL.
//!
//! Contient toute la logique de document, conversion et rendu d'équations.
//! Noyau PUR, sans egui : la frontière est imposée par le compilateur, car ce
//! crate ne déclare aucune dépendance UI. Le binaire egui (`mdall`) consomme
//! ce noyau. La logique est ainsi testable en isolation et réutilisable.
//!
//! Auto-suffisance : aucune ressource externe au runtime. Les polices Typst et
//! les assets KaTeX sont embarqués via `include_bytes!`/`include_str!`.

pub mod bibliography;
pub mod convert;
pub mod crossref;
pub mod docx_review;
pub mod editor;
pub mod equation_renderer;
pub mod export;
pub mod export_chromium;
pub mod export_formats;
pub mod figure_embed;
pub mod fonts;
pub mod export_typst;
pub mod import;
pub(crate) mod import_xml;
pub mod inline_math;
pub mod latex_macros;
pub mod render;
pub mod source_embed;
pub mod spell;
pub mod stats;
pub mod text_encoding;
// NOTE: `wysiwyg` is UI (egui LayoutJob building) and lives in the binary, not here.

#[cfg(test)]
mod tests_conversion;
