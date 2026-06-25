//! Plain UI state structs used by `MdApp` (no egui types, no logic).
//! Fields are `pub` because `MdApp` (in the crate root) constructs and reads
//! them across the module boundary.

use crate::output_format::OutputFormat;
use std::path::PathBuf;

/// Three-phase state machine for the Conversion Hub.
#[derive(PartialEq, Clone, Copy)]
#[allow(dead_code)] // FormatPick reserved for the conversion-grid phase
pub enum HubPhase {
    /// No file loaded - show drop zone + Browse button.
    Idle,
    /// At least one file loaded - show file list + actions.
    FileReady,
    /// User clicked Convert - show format grid.
    FormatPick,
}

/// Per-file status in the conversion queue.
#[derive(PartialEq, Clone, Copy)]
pub enum FileStatus {
    /// Loaded, not yet converted.
    Pending,
    /// Converted successfully.
    Done,
    /// Conversion failed (see `message`).
    Failed,
}

/// One file loaded into the hub (drag&drop or browse).
pub struct HubFile {
    pub path:        PathBuf,
    pub status:      FileStatus,
    /// Output path of the last successful conversion of this file.
    pub output_path: Option<PathBuf>,
    /// Per-file status / error message.
    pub message:     String,
    /// Per-file target format override. `None` = use the batch target.
    pub target:      Option<OutputFormat>,
}

impl HubFile {
    pub fn new(path: PathBuf) -> Self {
        Self {
            path,
            status: FileStatus::Pending,
            output_path: None,
            message: String::new(),
            target: None,
        }
    }
}

/// Conversion Hub runtime state (multi-file + batch queue).
pub struct ConversionHub {
    pub phase:        HubPhase,
    /// All loaded files (one = single-file flow, many = batch flow).
    pub files:        Vec<HubFile>,
    /// Markdown source after import of the single file (for "Open in Editor").
    pub converted_md: Option<String>,
    /// Index of the file whose per-file options panel is expanded.
    pub selected:     Option<usize>,
    /// Chosen output format for the batch (per-file `target` overrides it).
    pub batch_target: Option<OutputFormat>,
    /// True while the format-picker grid is shown.
    pub pick_format:  bool,
    /// Global status / error message.
    pub status:       String,
    pub is_error:     bool,
    /// True while a file is being dragged over the window.
    pub hovering:     bool,
    /// True while the batch queue is being processed (one file per frame).
    pub converting:   bool,
    /// Next file index to process in the batch queue.
    pub queue_index:  usize,
}

impl Default for ConversionHub {
    fn default() -> Self {
        Self {
            phase:        HubPhase::Idle,
            files:        Vec::new(),
            converted_md: None,
            selected:     None,
            batch_target: None,
            pick_format:  false,
            status:       String::new(),
            is_error:     false,
            hovering:     false,
            converting:   false,
            queue_index:  0,
        }
    }
}

/// Conversion output path settings.
pub struct ConversionSettings {
    /// false = SaveAs dialog (default), true = auto-save next to source.
    pub auto_save: bool,
    /// false = suffix (default), true = prefix.
    pub use_prefix: bool,
    /// Affix string added to the output filename. Default: "MDALL".
    pub affix: String,
}

impl Default for ConversionSettings {
    fn default() -> Self {
        Self { auto_save: false, use_prefix: false, affix: "MDALL".into() }
    }
}

pub struct EquationEditor {
    pub visible: bool,
    pub latex: String,
    /// Display equation: block index.  Inline equation: unused (0).
    pub index: usize,
    /// True when editing an inline $...$ or \(...\) equation.
    pub is_inline: bool,
    /// Inline only: byte range of the containing Paragraph block in source.
    pub inline_block_range: std::ops::Range<usize>,
    /// Inline only: original opening delimiter ("$" or "\\(").
    pub inline_delim_open: String,
    /// Inline only: original closing delimiter ("$" or "\\)").
    pub inline_delim_close: String,
    /// Inline only: original latex content before editing (used to evict old texture on Apply).
    pub inline_orig_latex: String,
    /// Inline only: index of the clicked run inside the paragraph's Vec<InlineRun>.
    pub inline_run_idx: usize,
}

/// Inline format state at the WYSIWYG cursor - updated every frame.
/// Used to highlight toolbar buttons and detect current formatting.
#[derive(Default, Clone, Copy)]
pub struct WysiwygFormatState {
    pub bold:          bool,
    pub italic:        bool,
    pub code:          bool,
    pub strikethrough: bool,
    /// 0 = not inside a heading; 1-6 = heading level at cursor.
    pub heading:       u8,
}

pub struct LinkDialog {
    pub visible: bool,
    pub text: String,
    pub url: String,
    pub is_image: bool,
}

/// Comment-authoring dialog: create a Review comment anchored to selected text.
pub struct CommentDialog {
    pub visible: bool,
    /// The selected passage the comment refers to (shown read-only as the anchor).
    pub anchor: String,
    /// The comment body being typed.
    pub body: String,
}

pub struct ExportDialog {
    pub visible: bool,
}

/// Properties popup for editing a standalone image (alt / url / width / align).
/// `replace` is the source byte range of the image block to overwrite on Apply.
pub struct ImageDialog {
    pub visible: bool,
    pub alt: String,
    pub url: String,
    /// Width in px as typed; empty = auto (no width attribute).
    pub width: String,
    pub align: crate::ui::editor::ImgAlign,
    pub replace: std::ops::Range<usize>,
}

/// Editor rendering mode, switchable in the Options panel.
/// Default is the continuous segmented flow; Block is the legacy click-to-edit model.
#[derive(PartialEq, Clone, Copy)]
pub enum EditorMode {
    /// Default: continuous segmented flow (text runs + inline rendered equations).
    SegmentedFlow,
    /// Block model: click a block to open an inline source editor.
    Block,
}

impl Default for EditorMode {
    fn default() -> Self { EditorMode::SegmentedFlow }
}

/// Severity of a transient toast notification.
#[derive(PartialEq, Clone, Copy)]
#[allow(dead_code)] // toast notification API, not all severities wired yet
pub enum ToastKind {
    Success,
    Error,
    Info,
}

/// A transient, auto-dismissing notification (shown bottom-right of the window).
pub struct Toast {
    pub message: String,
    pub kind: ToastKind,
    /// Seconds left before auto-dismiss; decremented by the frame delta each update.
    pub remaining: f32,
}

#[allow(dead_code)] // toast constructors, wired incrementally
impl Toast {
    /// Default on-screen lifetime, in seconds.
    pub const DEFAULT_SECS: f32 = 4.0;

    pub fn new(message: impl Into<String>, kind: ToastKind) -> Self {
        Self { message: message.into(), kind, remaining: Self::DEFAULT_SECS }
    }
    pub fn success(message: impl Into<String>) -> Self { Self::new(message, ToastKind::Success) }
    pub fn error(message: impl Into<String>)   -> Self { Self::new(message, ToastKind::Error) }
    pub fn info(message: impl Into<String>)     -> Self { Self::new(message, ToastKind::Info) }
}
