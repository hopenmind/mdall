//! File operations, editing commands, keyboard shortcuts and search.
//! Methods on MdApp, extracted from main.rs.

use eframe::egui;
use crate::MdApp;
use crate::ViewMode;
use crate::ui::state::LinkDialog;
use crate::{char_to_byte_index, byte_to_char_index};
use mdall_core::{export, source_embed};
use crate::i18n::t;

/// Shared status of a background dictionary download (thread → UI poll).
pub(crate) struct DictDownload {
    pub lang: String,
    pub done: bool,
    pub error: Option<String>,
}

/// `<exe-dir>/dictionaries/` - where bundled and downloaded dictionaries live.
fn dict_dir() -> Option<std::path::PathBuf> {
    std::env::current_exe()
        .ok()
        .and_then(|p| p.parent().map(|d| d.join("dictionaries")))
}

/// True when both `{lang}.dic` and `{lang}.aff` are present on disk.
fn dict_downloaded(lang: &str) -> bool {
    dict_dir()
        .map(|d| {
            d.join(format!("{lang}.dic")).exists() && d.join(format!("{lang}.aff")).exists()
        })
        .unwrap_or(false)
}

impl MdApp {
    pub(crate) fn handle_shortcuts(&mut self, ctx: &egui::Context) {
        let mut should_quit = false;
        ctx.input_mut(|i| {
            // ── Pinch / two-finger touchpad zoom (no modifier required) ────
            let pinch = i.zoom_delta();
            if (pinch - 1.0).abs() > 0.001 {
                self.zoom_level = (self.zoom_level * pinch).clamp(0.5, 3.0);
            }

            // ── Ctrl + scroll wheel → zoom (consume scroll so panel doesn't also scroll)
            if i.modifiers.ctrl && i.smooth_scroll_delta.y != 0.0 {
                let delta = i.smooth_scroll_delta.y * 0.002;
                self.zoom_level = (self.zoom_level + delta).clamp(0.5, 3.0);
                i.smooth_scroll_delta = egui::Vec2::ZERO;
            }

            if i.modifiers.ctrl {
                // ── File ──────────────────────────────────────────────────
                if i.key_pressed(egui::Key::N) { self.do_new(); }
                if i.key_pressed(egui::Key::O) { self.do_open(); }
                if i.key_pressed(egui::Key::S) {
                    if i.modifiers.shift { self.do_save_as(); } else { self.do_save(); }
                }
                if i.key_pressed(egui::Key::P) {
                    // Ctrl+Shift+P → command palette; Ctrl+P → print.
                    if i.modifiers.shift {
                        self.command_palette_open = true;
                        self.palette_query.clear();
                    } else {
                        self.do_print();
                    }
                }
                if i.key_pressed(egui::Key::Q) { should_quit = true; }

                // ── App ───────────────────────────────────────────────────
                // Ctrl+Shift+D → toggle light / dark theme (light stays the default).
                if i.modifiers.shift && i.key_pressed(egui::Key::D) {
                    self.dark_mode = !self.dark_mode;
                }

                // ── Formatting ────────────────────────────────────────────
                if i.key_pressed(egui::Key::B) { self.wrap_text("**", "**"); }
                if i.key_pressed(egui::Key::I) { self.wrap_text("*", "*"); }
                if i.key_pressed(egui::Key::U) { self.wrap_text("<u>", "</u>"); }

                // ── Insert ────────────────────────────────────────────────
                if i.key_pressed(egui::Key::K) {
                    self.open_link_dialog(false);
                }
                if i.key_pressed(egui::Key::E) {
                    self.insert_text("$$\n\\sum_{i=0}^{n} x_i\n$$\n");
                }

                // ── Search / Find ─────────────────────────────────────────
                if i.key_pressed(egui::Key::F) {
                    self.show_search = true;
                    self.search_show_replace = false;
                    self.compute_search_matches();
                }
                if i.key_pressed(egui::Key::H) {
                    self.show_search = true;
                    self.search_show_replace = true;
                    self.compute_search_matches();
                }

                // ── View modes ────────────────────────────────────────────
                if i.key_pressed(egui::Key::Num1) { self.view_mode = ViewMode::Source; }
                if i.key_pressed(egui::Key::Num2) { self.view_mode = ViewMode::Split; }
                if i.key_pressed(egui::Key::Num3) {
                    self.view_mode = ViewMode::Editor;
                    self.segments_dirty = true;
                }

                // ── Zoom keyboard ─────────────────────────────────────────
                if i.key_pressed(egui::Key::Equals) || i.key_pressed(egui::Key::Plus) {
                    self.zoom_level = (self.zoom_level + 0.1).min(3.0);
                }
                if i.key_pressed(egui::Key::Minus) {
                    self.zoom_level = (self.zoom_level - 0.1).max(0.5);
                }
                if i.key_pressed(egui::Key::Num0) {
                    self.zoom_level = 1.0;
                }
            }
            // ── F3 / Shift+F3 - find next / previous (no Ctrl required) ──
            if i.key_pressed(egui::Key::F3) {
                if i.modifiers.shift { self.do_find_prev(); } else { self.do_find_next(); }
            }
            // ── Escape - close search bar ─────────────────────────────────
            if i.key_pressed(egui::Key::Escape) && self.show_search {
                self.show_search = false;
            }
        });
        // Quit handled outside the closure (ctx is borrowed inside)
        if should_quit {
            ctx.send_viewport_cmd(egui::ViewportCommand::Close);
        }
    }

    /// Re-import a DOCX previously exported by MD -> ALL.
    /// Recovers the original markdown (+ LaTeX) from the embedded source entry.
    pub(crate) fn do_import_docx(&mut self) {
        if let Some(path) = rfd::FileDialog::new()
            .add_filter("Word Document", &["docx"])
            .set_title("Import DOCX (MD -> ALL export)")
            .pick_file()
        {
            match source_embed::import_docx_source(&path) {
                Ok(markdown) => {
                    self.source = markdown;
                    self.current_file = None; // unsaved - prompt on next save
                    self.modified = true;
                    self.segments_dirty = true;
                    self.view_mode = ViewMode::Editor;
                    // Surface any reviewer feedback (tracked changes + comments) the
                    // supervisor left in Word, so it can be read in-app.
                    self.review_items =
                        mdall_core::docx_review::extract_review_items(&path).unwrap_or_default();
                    self.show_review_panel = !self.review_items.is_empty();
                    let review_note = if self.review_items.is_empty() {
                        String::new()
                    } else {
                        format!(" - {} review item(s)", self.review_items.len())
                    };
                    self.status_msg = format!(
                        "Imported from \u{201C}{}\u{201D} - {} chars recovered{}",
                        path.file_name().unwrap_or_default().to_string_lossy(),
                        self.source.len(),
                        review_note,
                    );
                }
                Err(e) => {
                    self.status_msg = format!("Import failed: {}", e);
                }
            }
        }
    }

    pub(crate) fn do_new(&mut self) {
        self.source.clear();
        self.current_file = None;
        self.modified = false;
        self.segments_dirty = true;
        self.status_msg = "New file".into();
    }

    pub(crate) fn do_open(&mut self) {
        if let Some(path) = rfd::FileDialog::new()
            .add_filter("All Supported", &[
                "md","markdown","txt","docx","html","htm","epub","odt","rtf",
                "tex","latex","org","rst","wiki","mediawiki","adoc","asciidoc","asc","typ",
                "ipynb","bib","fb2","pptx","eml","csv","tsv","rmd","qmd",
                "py","js","ts","rs","c","cpp","java","go","rb","php","sh","r",
            ])
            .add_filter("Markdown",            &["md","markdown"])
            .add_filter("Word Document",       &["docx"])
            .add_filter("HTML",                &["html","htm"])
            .add_filter("EPUB eBook",          &["epub"])
            .add_filter("OpenDocument",        &["odt"])
            .add_filter("Rich Text (RTF)",     &["rtf"])
            .add_filter("LaTeX",               &["tex","latex"])
            .add_filter("Org-mode",            &["org"])
            .add_filter("reStructuredText",    &["rst"])
            .add_filter("AsciiDoc",            &["adoc","asciidoc","asc"])
            .add_filter("Typst",               &["typ"])
            .add_filter("Jupyter Notebook",    &["ipynb"])
            .add_filter("BibTeX",              &["bib"])
            .add_filter("FictionBook",         &["fb2"])
            .add_filter("PowerPoint",          &["pptx"])
            .add_filter("Email",               &["eml"])
            .add_filter("CSV / TSV",           &["csv","tsv"])
            .add_filter("R Markdown / Quarto", &["rmd","qmd"])
            .add_filter("Source Code",         &["py","js","ts","rs","c","cpp","java","go","rb","php","sh","r"])
            .add_filter("Plain Text",          &["txt"])
            .add_filter("All Files",           &["*"])
            .pick_file()
        {
            match Self::import_to_md(&path) {
                Ok(content) => {
                    self.source = content;
                    self.current_file = Some(path);
                    self.modified = false;
                    self.segments_dirty = true;
                    self.status_msg = "Opened".into();
                    // Switch to Editor so user sees the imported content
                    if self.view_mode == ViewMode::Converter {
                        self.view_mode = ViewMode::Editor;
                    }
                }
                Err(e) => self.status_msg = format!("Import error: {}", e),
            }
        }
    }

    pub(crate) fn do_save(&mut self) {
        if let Some(ref path) = self.current_file.clone() {
            match std::fs::write(path, &self.source) {
                Ok(()) => { self.modified = false; self.status_msg = "Saved".into(); }
                Err(e) => self.status_msg = format!("Save error: {}", e),
            }
        } else {
            self.do_save_as();
        }
    }

    pub(crate) fn do_save_as(&mut self) {
        // Install this document's custom LaTeX macros so the equation renderers
        // (KaTeX/Typst) can expand them on export.
        mdall_core::latex_macros::install_from_source(&self.source);
        let mut dlg = rfd::FileDialog::new()
            .add_filter("Markdown", &["md"])
            .add_filter("PDF", &["pdf"])
            .add_filter("HTML", &["html", "htm"])
            .add_filter("All Files", &["*"]);
        if let Some(ref f) = self.current_file {
            if let Some(dir) = f.parent() { dlg = dlg.set_directory(dir); }
        }
        if let Some(path) = dlg.save_file()
        {
            let ext = path.extension().and_then(|e| e.to_str()).unwrap_or("md").to_lowercase();
            match ext.as_str() {
                "pdf" => {
                    let metadata = self.meta.clone();
                    let source_dir = self.current_file.as_ref().and_then(|f| f.parent());
                    match export::export_pdf(&self.source, &path, &metadata, source_dir) {
                        Ok(()) if path.exists() => { self.status_msg = "PDF exported".into(); let _ = open::that(&path); }
                        Ok(()) => self.status_msg = "PDF error: file not created".into(),
                        Err(e) => self.status_msg = format!("PDF error: {}", e),
                    }
                }
                "html" | "htm" => {
                    let metadata = self.meta.clone();
                    let source_dir = self.current_file.as_ref().and_then(|f| f.parent());
                    match export::export_html(&self.source, &path, &metadata, source_dir) {
                        Ok(()) if path.exists() => { self.status_msg = "HTML exported".into(); let _ = open::that(&path); }
                        Ok(()) => self.status_msg = "HTML error: file not created".into(),
                        Err(e) => self.status_msg = format!("HTML error: {}", e),
                    }
                }
                _ => {
                    match std::fs::write(&path, &self.source) {
                        Ok(()) => {
                            self.current_file = Some(path);
                            self.modified = false;
                            self.status_msg = "Saved".into();
                        }
                        Err(e) => self.status_msg = format!("Save error: {}", e),
                    }
                }
            }
        }
    }

    pub(crate) fn do_export_pdf(&mut self) {
        mdall_core::latex_macros::install_from_source(&self.source);
        let mut dlg = rfd::FileDialog::new().add_filter("PDF", &["pdf"]);
        if let Some(ref f) = self.current_file {
            if let Some(dir) = f.parent() { dlg = dlg.set_directory(dir); }
            if let Some(stem) = f.file_stem() {
                dlg = dlg.set_file_name(&format!("{}.pdf", stem.to_string_lossy()));
            }
        }
        if let Some(path) = dlg.save_file() {
            let metadata = self.meta.clone();
            let source_dir = self.current_file.as_ref().and_then(|f| f.parent());
            match export::export_pdf(&self.source, &path, &metadata, source_dir) {
                Ok(()) if path.exists() => { self.status_msg = "PDF exported".into(); let _ = open::that(&path); }
                Ok(()) => self.status_msg = "PDF error: file not created".into(),
                Err(e) => self.status_msg = format!("PDF error: {}", e),
            }
        }
    }

    pub(crate) fn do_export_html(&mut self) {
        mdall_core::latex_macros::install_from_source(&self.source);
        let mut dlg = rfd::FileDialog::new().add_filter("HTML", &["html"]);
        if let Some(ref f) = self.current_file {
            if let Some(dir) = f.parent() { dlg = dlg.set_directory(dir); }
            if let Some(stem) = f.file_stem() {
                dlg = dlg.set_file_name(&format!("{}.html", stem.to_string_lossy()));
            }
        }
        if let Some(path) = dlg.save_file() {
            let metadata = self.meta.clone();
            let source_dir = self.current_file.as_ref().and_then(|f| f.parent());
            match export::export_html(&self.source, &path, &metadata, source_dir) {
                Ok(()) if path.exists() => { self.status_msg = "HTML exported".into(); let _ = open::that(&path); }
                Ok(()) => self.status_msg = "HTML error: file not created".into(),
                Err(e) => self.status_msg = format!("HTML error: {}", e),
            }
        }
    }

    pub(crate) fn do_insert_image_file(&mut self) {
        if let Some(path) = rfd::FileDialog::new()
            .add_filter("Images", &["png", "jpg", "jpeg", "gif", "svg", "webp"])
            .pick_file()
        {
            self.insert_text(&format!("![image]({})", path.display()));
        }
    }

    /// Open the link/image dialog, pre-filling the text field with the current
    /// selection so wrapping selected text as a hyperlink keeps that text (the
    /// dialog's Insert replaces the selection with `[text](url)`; an empty text
    /// field would otherwise erase the selected words).
    pub(crate) fn open_link_dialog(&mut self, is_image: bool) {
        let s = self.cursor_pos.min(self.selection_anchor);
        let e = self.cursor_pos.max(self.selection_anchor);
        let text = if s != e {
            let bs = char_to_byte_index(&self.source, s).min(self.source.len());
            let be = char_to_byte_index(&self.source, e).min(self.source.len());
            self.source[bs..be].to_string()
        } else {
            String::new()
        };
        self.link_dialog = LinkDialog { visible: true, text, url: String::new(), is_image };
    }

    pub(crate) fn insert_text(&mut self, text: &str) {
        let sel_start  = self.cursor_pos.min(self.selection_anchor);
        let sel_end    = self.cursor_pos.max(self.selection_anchor);
        let byte_start = char_to_byte_index(&self.source, sel_start).min(self.source.len());
        let byte_end   = char_to_byte_index(&self.source, sel_end).min(self.source.len());

        self.source.replace_range(byte_start..byte_end, text);
        let new_pos = sel_start + text.chars().count();
        self.cursor_pos    = new_pos;
        self.selection_anchor = new_pos;

        self.apply_cursor_to_editor_state();
        self.modified = true;
        self.segments_dirty = true;
        self.status_msg = "Modified".into();
    }


    /// Schedule a cursor move for the next frame (applied inside show_source_editor).
    pub(crate) fn apply_cursor_to_editor_state(&mut self) {
        self.pending_cursor = Some((self.cursor_pos, self.selection_anchor));
    }

    /// Wraps the current cursor position (or selected text) in an HTML alignment div.
    /// `align` is one of: left | center | right | justify.
    pub(crate) fn wrap_block_align(&mut self, align: &str) {
        self.wrap_text(
            &format!("<div style=\"text-align:{}\">\n\n", align),
            "\n\n</div>",
        );
    }

    /// Wrap selected text with `before`/`after` markers.
    /// If text is selected, the selection is wrapped: "hello" → "**hello**".
    /// If nothing is selected, inserts "before·text·after" and selects the placeholder.
    pub(crate) fn wrap_text(&mut self, before: &str, after: &str) {
        let sel_start = self.cursor_pos.min(self.selection_anchor);
        let sel_end   = self.cursor_pos.max(self.selection_anchor);
        let has_sel   = sel_start != sel_end;

        let byte_start = char_to_byte_index(&self.source, sel_start).min(self.source.len());
        let byte_end   = char_to_byte_index(&self.source, sel_end).min(self.source.len());

        let inner = if has_sel {
            self.source[byte_start..byte_end].to_string()
        } else {
            "text".to_string()
        };

        let replacement = format!("{}{}{}", before, inner, after);
        self.source.replace_range(byte_start..byte_end, &replacement);

        // Position cursor after the closing marker; select the inner text if placeholder.
        let new_end = sel_start + before.chars().count() + inner.chars().count() + after.chars().count();
        self.cursor_pos    = new_end;
        self.selection_anchor = if has_sel {
            new_end
        } else {
            // Select the placeholder "text" so the user can immediately retype it.
            sel_start + before.chars().count()
        };

        self.apply_cursor_to_editor_state();
        self.modified = true;
        self.segments_dirty = true;
        self.status_msg = "Modified".into();
    }

    /// Recompute all match positions in `source` for the current query + case setting.
    pub(crate) fn compute_search_matches(&mut self) {
        self.search_matches.clear();
        if self.search_query.is_empty() { return; }

        let (haystack, needle) = if self.search_case_sensitive {
            (self.source.clone(), self.search_query.clone())
        } else {
            (self.source.to_lowercase(), self.search_query.to_lowercase())
        };

        let needle_len = needle.len();
        if needle_len == 0 { return; }
        let mut start = 0usize;
        while start < haystack.len() {
            match haystack[start..].find(&needle) {
                Some(rel) => {
                    self.search_matches.push(start + rel);
                    start += rel + needle_len;
                }
                None => break,
            }
        }
    }

    /// Jump source cursor to the current match and select it.
    /// In Preview mode → switch to Split so the user can see the match in context.
    pub(crate) fn jump_to_match(&mut self) {
        let Some(&byte_start) = self.search_matches.get(self.search_match_idx) else {
            self.status_msg = "No match".into();
            return;
        };
        let byte_end = (byte_start + self.search_query.len()).min(self.source.len());
        let char_start = byte_to_char_index(&self.source, byte_start);
        let char_end   = byte_to_char_index(&self.source, byte_end);

        self.cursor_pos       = char_end;
        self.selection_anchor = char_start;
        self.apply_cursor_to_editor_state();

        // In Editor-only mode, switch to Split so the source match is visible
        if self.view_mode == ViewMode::Editor {
            self.view_mode = ViewMode::Split;
        }
        self.request_source_focus = true;

        let total = self.search_matches.len();
        self.status_msg = format!(
            "Match {} of {}",
            self.search_match_idx + 1,
            total,
        );
    }

    pub(crate) fn do_find_next(&mut self) {
        if self.search_query.is_empty() { return; }
        self.compute_search_matches();
        if self.search_matches.is_empty() {
            self.status_msg = "Not found".into();
            return;
        }
        // Advance past current match (wrap around)
        self.search_match_idx = (self.search_match_idx + 1) % self.search_matches.len();
        self.jump_to_match();
    }

    pub(crate) fn do_find_prev(&mut self) {
        if self.search_query.is_empty() { return; }
        self.compute_search_matches();
        if self.search_matches.is_empty() {
            self.status_msg = "Not found".into();
            return;
        }
        let len = self.search_matches.len();
        self.search_match_idx = if self.search_match_idx == 0 { len - 1 } else { self.search_match_idx - 1 };
        self.jump_to_match();
    }

    /// Replace the currently highlighted occurrence and jump to the next match.
    pub(crate) fn do_replace_current(&mut self) {
        if self.search_matches.is_empty() { return; }
        let byte_start = self.search_matches[self.search_match_idx];
        let byte_end   = (byte_start + self.search_query.len()).min(self.source.len());
        self.source.replace_range(byte_start..byte_end, &self.replace_query);
        self.modified       = true;
        self.segments_dirty = true;
        self.compute_search_matches();
        // Keep index in bounds after replacement
        if !self.search_matches.is_empty() {
            self.search_match_idx = self.search_match_idx.min(self.search_matches.len() - 1);
            self.jump_to_match();
        } else {
            self.status_msg = "All occurrences replaced".into();
        }
    }

    pub(crate) fn do_replace_all(&mut self) {
        if self.search_query.is_empty() { return; }
        // Compute fresh matches with current case setting
        self.compute_search_matches();
        let count = self.search_matches.len();
        if count == 0 {
            self.status_msg = "Not found".into();
            return;
        }
        // Replace from end to start to preserve byte offsets
        for &byte_start in self.search_matches.iter().rev() {
            let byte_end = (byte_start + self.search_query.len()).min(self.source.len());
            self.source.replace_range(byte_start..byte_end, &self.replace_query);
        }
        self.modified       = true;
        self.segments_dirty = true;
        self.search_matches.clear();
        self.search_match_idx = 0;
        self.status_msg = format!("Replaced {} occurrence{}", count, if count == 1 { "" } else { "s" });
    }
}

impl MdApp {
    /// Export document to a temp PDF then send it to the system default printer.
    pub(crate) fn do_print(&mut self) {
        mdall_core::latex_macros::install_from_source(&self.source);
        let tmp_path = std::env::temp_dir().join("mdall-print.pdf");
        let metadata  = self.meta.clone();
        let source_dir = self.current_file.as_ref().and_then(|f| f.parent());

        match export::export_pdf(&self.source, &tmp_path, &metadata, source_dir) {
            Ok(()) if tmp_path.exists() => {
                // Use the PDF verb so Windows opens the default PDF printer dialog.
                let path_str = tmp_path.to_string_lossy().replace('\'', "''"); // escape for PS string
                let cmd = format!("Start-Process -FilePath '{}' -Verb Print", path_str);
                match std::process::Command::new("powershell")
                    .args(["-NoProfile", "-WindowStyle", "Hidden", "-Command", &cmd])
                    .spawn()
                {
                    Ok(_) => self.status_msg = "Sent to printer".into(),
                    Err(e) => self.status_msg = format!("Print error: {}", e),
                }
            }
            Ok(()) => self.status_msg = "Print error: PDF not created".into(),
            Err(e) => self.status_msg = format!("Print error: {}", e),
        }
    }
}

impl MdApp {
    /// Right-hand panel listing reviewer feedback (tracked changes + comments)
    /// recovered from an imported DOCX. Read-only display + jump-to-source; it
    /// never mutates the document source.
    pub(crate) fn render_review_panel(&mut self, ctx: &egui::Context) {
        if !self.show_review_panel || self.review_items.is_empty() {
            return;
        }
        use mdall_core::docx_review::ReviewKind;
        let mut jump: Option<String> = None;
        let mut dismiss: Option<usize> = None;
        let mut close = false;
        let mut hover_anchor: Option<String> = None;
        let mut click_anchor: Option<String> = None;
        egui::SidePanel::right("review_panel")
            .resizable(true)
            .default_width(300.0)
            .min_width(220.0)
            .show(ctx, |ui| {
                ui.add_space(6.0);
                ui.horizontal(|ui| {
                    ui.label(egui::RichText::new("Review").heading());
                    ui.label(egui::RichText::new(format!("({})", self.review_items.len())).weak());
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        if ui.button("\u{2715}").on_hover_text("Close panel").clicked() {
                            close = true;
                        }
                    });
                });
                ui.label(egui::RichText::new("Tracked changes & comments from the imported DOCX")
                    .small().weak());
                ui.separator();
                egui::ScrollArea::vertical().show(ui, |ui| {
                    for (i, it) in self.review_items.iter().enumerate() {
                        let (tag, col) = match it.kind {
                            ReviewKind::Insertion => ("INSERT", egui::Color32::from_rgb(39, 174, 96)),
                            ReviewKind::Deletion  => ("DELETE", egui::Color32::from_rgb(192, 57, 43)),
                            ReviewKind::Comment   => ("COMMENT", egui::Color32::from_rgb(41, 128, 185)),
                        };
                        // The passage this item is anchored to (for the editor frame).
                        let anchor = if it.context.is_empty() { it.text.clone() } else { it.context.clone() };
                        let marked = self.review_mark.as_deref() == Some(anchor.as_str());
                        let mut frame = egui::Frame::group(ui.style());
                        if marked {
                            // Persistent "marked" item: amber tint + border.
                            frame = frame
                                .fill(egui::Color32::from_rgba_unmultiplied(201, 146, 10, 28))
                                .stroke(egui::Stroke::new(1.0, crate::theme::ACCENT));
                        }
                        let ir = frame.show(ui, |ui| {
                            ui.horizontal(|ui| {
                                ui.label(egui::RichText::new(tag).small().strong().color(col));
                                if !it.author.is_empty() {
                                    ui.label(egui::RichText::new(&it.author).small().weak());
                                }
                            });
                            if it.kind == ReviewKind::Comment && !it.context.is_empty() {
                                ui.label(egui::RichText::new(format!("\u{201C}{}\u{201D}", it.context))
                                    .small().italics().weak());
                            }
                            let body = if it.kind == ReviewKind::Deletion {
                                egui::RichText::new(&it.text).strikethrough()
                            } else {
                                egui::RichText::new(&it.text)
                            };
                            ui.label(body);
                            ui.horizontal(|ui| {
                                if ui.small_button("Jump to text").clicked() {
                                    jump = Some(anchor.clone());
                                }
                                if ui.small_button("Dismiss").clicked() {
                                    dismiss = Some(i);
                                }
                            });
                        });
                        // Hover frames the passage; clicking the card marks it.
                        let resp = ir.response.interact(egui::Sense::click());
                        if resp.hovered() {
                            hover_anchor = Some(anchor.clone());
                            ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
                        }
                        if resp.clicked() {
                            click_anchor = Some(anchor.clone());
                        }
                        ui.add_space(4.0);
                    }
                });
            });
        if close {
            self.show_review_panel = false;
        }
        // Hover frames the anchored passage in the editor this frame; a click
        // toggles a persistent mark on it.
        self.review_hl = hover_anchor;
        if let Some(a) = click_anchor {
            self.review_mark = if self.review_mark.as_deref() == Some(a.as_str()) {
                None
            } else {
                Some(a)
            };
        }
        if let Some(t) = jump {
            self.jump_to_text(&t);
        }
        if let Some(i) = dismiss {
            if i < self.review_items.len() {
                self.review_items.remove(i);
            }
            if self.review_items.is_empty() {
                self.show_review_panel = false;
            }
        }
    }

    /// Open the comment-authoring dialog anchored to the given selected passage.
    pub(crate) fn open_comment_dialog(&mut self, anchor: String) {
        self.comment_dialog = crate::ui::state::CommentDialog {
            visible: true,
            anchor,
            body: String::new(),
        };
    }

    /// Modal to write a comment anchored to a selected passage. On Add it appends
    /// a `Comment` review item (author "You") and opens the Review panel, so the
    /// new comment behaves exactly like an imported one (hover = frame, etc.).
    pub(crate) fn show_comment_dialog(&mut self, ctx: &egui::Context) {
        if !self.comment_dialog.visible {
            return;
        }
        let mut add = false;
        let mut cancel = false;
        egui::Window::new("Add comment")
            .id(egui::Id::new("comment_dialog"))
            .collapsible(false)
            .resizable(false)
            .anchor(egui::Align2::CENTER_CENTER, egui::vec2(0.0, 0.0))
            .show(ctx, |ui| {
                ui.set_min_width(360.0);
                ui.label(egui::RichText::new("Comment on this passage:").small().weak());
                let preview: String = self.comment_dialog.anchor.chars().take(140).collect();
                let ellipsis = if self.comment_dialog.anchor.chars().count() > 140 { "..." } else { "" };
                ui.label(egui::RichText::new(format!("\u{201C}{preview}{ellipsis}\u{201D}")).italics());
                ui.add_space(6.0);
                ui.add(
                    egui::TextEdit::multiline(&mut self.comment_dialog.body)
                        .desired_rows(3)
                        .desired_width(f32::INFINITY)
                        .hint_text("Your comment..."),
                );
                ui.add_space(8.0);
                ui.horizontal(|ui| {
                    if ui.button("Add").clicked() {
                        add = true;
                    }
                    if ui.button("Cancel").clicked() {
                        cancel = true;
                    }
                });
            });
        if add && !self.comment_dialog.body.trim().is_empty() {
            self.review_items.push(mdall_core::docx_review::ReviewItem {
                kind: mdall_core::docx_review::ReviewKind::Comment,
                author: "You".into(),
                date: chrono::Local::now().format("%Y-%m-%d").to_string(),
                text: self.comment_dialog.body.trim().to_string(),
                context: self.comment_dialog.anchor.clone(),
            });
            self.show_review_panel = true;
            self.comment_dialog.visible = false;
            self.status_msg = "Comment added".into();
        } else if cancel || (add && self.comment_dialog.body.trim().is_empty()) {
            self.comment_dialog.visible = false;
        }
    }

    /// Select the first occurrence of `needle` in the source and reveal it
    /// (switches Editor-only mode to Split so the source selection is visible).
    fn jump_to_text(&mut self, needle: &str) {
        if needle.is_empty() {
            return;
        }
        if let Some(b) = self.source.find(needle) {
            self.selection_anchor = byte_to_char_index(&self.source, b);
            self.cursor_pos = byte_to_char_index(&self.source, b + needle.len());
            if self.view_mode == ViewMode::Editor {
                self.view_mode = ViewMode::Split;
            }
            self.request_source_focus = true;
            self.apply_cursor_to_editor_state();
            self.status_msg = "Jumped to reviewed text".into();
        } else {
            self.status_msg = "Reviewed text not found in the recovered document".into();
        }
    }
}

impl MdApp {
    /// The Module system window: tabbed manager for the spell engine's
    /// dictionaries, the application language (i18n), and reserved slots for
    /// citation styles, themes and export templates (downloadable packs).
    pub(crate) fn show_module_window(&mut self, ctx: &egui::Context) {
        let mut open = self.module_open;
        egui::Window::new(t("module.title"))
            .id(egui::Id::new("modules_window"))
            .open(&mut open)
            .resizable(true)
            .default_width(540.0)
            .show(ctx, |ui| {
                ui.horizontal(|ui| {
                    // Tabs come from the module registry (single source of truth).
                    for (i, cat) in crate::modules::ModuleCategory::all().iter().enumerate() {
                        if ui.selectable_label(self.module_tab == i as u8, t(cat.title_key())).clicked() {
                            self.module_tab = i as u8;
                        }
                    }
                });
                ui.separator();
                ui.add_space(4.0);
                match self.module_tab {
                    0 => self.module_tab_dictionaries(ui),
                    1 => self.module_tab_language(ui),
                    _ => {
                        ui.add_space(8.0);
                        ui.label(egui::RichText::new(t("module.reserved")).strong());
                        ui.label(egui::RichText::new(t("module.reserved_hint")).small().weak());
                    }
                }
            });
        self.module_open = open;
    }

    fn module_tab_dictionaries(&mut self, ui: &mut egui::Ui) {
        if ui.checkbox(&mut self.spell_enabled, t("module.enable_spell")).changed() {
            self.segments_dirty = true; // (re)compute or clear issues
            self.show_spelling_panel = self.spell_enabled && self.spell.is_some();
        }
        ui.add_space(4.0);
        match &self.spell {
            Some(sc) => {
                ui.label(egui::RichText::new(
                    format!("{}: {}", t("module.active_dict"), sc.lang())
                ).strong());
            }
            None => {
                ui.label(egui::RichText::new(t("module.no_dict")).weak());
            }
        }
        ui.add_space(6.0);
        if ui.button(t("module.add_dict")).clicked() {
            self.load_dictionary_dialog();
        }
        ui.label(egui::RichText::new(t("module.add_dict_hint")).small().weak());

        ui.add_space(12.0);
        ui.label(egui::RichText::new(t("module.downloadable")).strong());
        ui.label(egui::RichText::new(t("module.downloadable_hint")).small().weak());
        if !self.dict_status.is_empty() {
            ui.label(egui::RichText::new(&self.dict_status).small().color(crate::theme::ACCENT));
        }
        ui.add_space(4.0);
        // State per row: downloading (spinner) → downloaded (green tick + Use) →
        // active (green "in use"). Buttons stay clickable; re-entry is guarded
        // inside start_dict_download.
        let downloading = self
            .dict_dl
            .as_ref()
            .and_then(|s| s.lock().ok().map(|x| x.lang.clone()));
        let active_dict = self.spell.as_ref().map(|sc| sc.lang().to_string());
        let mut to_dl: Option<(String, String)> = None;
        let mut to_use: Option<String> = None;
        for (tag, repo, label) in [
            ("en_US", "en", "English (US)"), ("en_GB", "en-GB", "English (UK)"),
            ("fr_FR", "fr", "Français"), ("de_DE", "de", "Deutsch"),
            ("es_ES", "es", "Español"), ("it_IT", "it", "Italiano"),
        ] {
            // Shared "downloadable resource" convention (modules::downloadable_row).
            let state = if downloading.as_deref() == Some(tag) {
                crate::modules::DlState::Downloading
            } else if active_dict.as_deref() == Some(tag) {
                crate::modules::DlState::Active
            } else if dict_downloaded(tag) {
                crate::modules::DlState::Installed
            } else {
                crate::modules::DlState::NotInstalled
            };
            match crate::modules::downloadable_row(ui, label, state) {
                crate::modules::DlAction::Download | crate::modules::DlAction::Redownload => {
                    to_dl = Some((tag.to_string(), repo.to_string()));
                }
                crate::modules::DlAction::Use => to_use = Some(tag.to_string()),
                crate::modules::DlAction::None => {}
            }
        }
        if let Some((tag, repo)) = to_dl {
            self.start_dict_download(&tag, &repo);
        }
        if let Some(tag) = to_use {
            self.use_downloaded_dictionary(&tag);
        }
    }

    /// Activate an already-downloaded dictionary (the green "Use" action).
    fn use_downloaded_dictionary(&mut self, lang: &str) {
        let Some(dir) = dict_dir() else { return };
        let dic = dir.join(format!("{lang}.dic"));
        let aff = dir.join(format!("{lang}.aff"));
        match (std::fs::read_to_string(&aff), std::fs::read_to_string(&dic)) {
            (Ok(a), Ok(d)) => match mdall_core::spell::SpellChecker::from_aff_dic(&a, &d, lang) {
                Ok(sc) => {
                    self.spell = Some(sc);
                    self.spell_enabled = true;
                    self.show_spelling_panel = true;
                    self.spell_sugg_cache.clear();
                    self.segments_dirty = true;
                    self.dict_status = format!("Using dictionary '{lang}'");
                    self.status_msg = format!("Dictionary '{lang}' active");
                }
                Err(e) => self.dict_status = format!("Failed to load '{lang}': {e}"),
            },
            _ => self.dict_status = format!("Could not read the '{lang}' files"),
        }
    }

    fn module_tab_language(&mut self, ui: &mut egui::Ui) {
        ui.label(egui::RichText::new(t("module.language")).strong());
        ui.label(egui::RichText::new(t("module.language_hint")).small().weak());
        ui.add_space(8.0);
        for (tag, name) in [
            ("en", "English"), ("fr", "Français"), ("de", "Deutsch"),
            ("es", "Español"), ("it", "Italiano"),
        ] {
            if ui.selectable_label(self.app_lang == tag, name).clicked() {
                self.app_lang = tag.to_string();
            }
        }
    }

    /// Pick a Hunspell `.dic` and load it together with its sibling `.aff`.
    fn load_dictionary_dialog(&mut self) {
        let Some(dic_path) = rfd::FileDialog::new()
            .add_filter("Hunspell dictionary", &["dic"])
            .set_title("Add a Hunspell dictionary (.dic)")
            .pick_file()
        else {
            return;
        };
        let aff_path = dic_path.with_extension("aff");
        let lang = dic_path.file_stem().and_then(|s| s.to_str()).unwrap_or("custom").to_string();
        match (std::fs::read_to_string(&aff_path), std::fs::read_to_string(&dic_path)) {
            (Ok(aff), Ok(dic)) => {
                match mdall_core::spell::SpellChecker::from_aff_dic(&aff, &dic, &lang) {
                    Ok(sc) => {
                        self.spell = Some(sc);
                        self.spell_enabled = true;
                        self.show_spelling_panel = true;
                        self.spell_sugg_cache.clear();
                        self.segments_dirty = true; // recompute issues against the new dict
                        self.status_msg = format!("Dictionary '{}' loaded", lang);
                    }
                    Err(e) => self.status_msg = format!("Dictionary error: {e}"),
                }
            }
            _ => {
                self.status_msg =
                    format!("Need both {}.dic and {}.aff in the same folder", lang, lang);
            }
        }
    }

    /// Fill the suggestion cache for the first `limit` issues that lack one.
    /// Disjoint-field borrows (`spell` vs `spell_sugg_cache`) keep this cheap and
    /// run only when new misspelled words appear, never per frame.
    fn ensure_spell_suggestions(&mut self, limit: usize) {
        let need: Vec<String> = self
            .spell_issues
            .iter()
            .take(limit)
            .map(|m| m.word.clone())
            .filter(|w| !self.spell_sugg_cache.contains_key(w))
            .collect();
        if need.is_empty() {
            return;
        }
        if let Some(sc) = self.spell.as_ref() {
            for w in need {
                let s = sc.suggest(&w);
                self.spell_sugg_cache.insert(w, s);
            }
        }
    }

    /// Right-hand panel listing spelling issues with clickable suggestions,
    /// "Add to dictionary" and jump-to-source. Edits go through the source only.
    pub(crate) fn render_spelling_panel(&mut self, ctx: &egui::Context) {
        if !self.show_spelling_panel || !self.spell_enabled || self.spell.is_none() {
            return;
        }
        self.ensure_spell_suggestions(120);

        let mut close = false;
        let mut jump: Option<(usize, usize)> = None;
        let mut replace: Option<(usize, usize, String)> = None;
        let mut add: Option<String> = None;

        egui::SidePanel::right("spelling_panel")
            .resizable(true)
            .default_width(280.0)
            .min_width(200.0)
            .show(ctx, |ui| {
                ui.add_space(6.0);
                ui.horizontal(|ui| {
                    ui.label(egui::RichText::new(t("panel.spelling")).heading());
                    ui.label(egui::RichText::new(format!("({})", self.spell_issues.len())).weak());
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        if ui.button("\u{2715}").clicked() {
                            close = true;
                        }
                    });
                });
                if let Some(sc) = &self.spell {
                    ui.label(egui::RichText::new(
                        format!("{}: {}", t("panel.dictionary"), sc.lang())
                    ).small().weak());
                }
                ui.separator();
                if self.spell_issues.is_empty() {
                    ui.add_space(8.0);
                    ui.label(egui::RichText::new(t("panel.no_issues")).weak());
                    return;
                }
                egui::ScrollArea::vertical().show(ui, |ui| {
                    for m in self.spell_issues.iter().take(120) {
                        egui::Frame::group(ui.style()).show(ui, |ui| {
                            ui.label(egui::RichText::new(&m.word)
                                .strong()
                                .color(egui::Color32::from_rgb(192, 57, 43)));
                            if let Some(sugg) = self.spell_sugg_cache.get(&m.word) {
                                if sugg.is_empty() {
                                    ui.label(egui::RichText::new(t("panel.no_suggestions")).small().weak());
                                } else {
                                    ui.horizontal_wrapped(|ui| {
                                        for s in sugg.iter().take(5) {
                                            if ui.small_button(s).clicked() {
                                                replace = Some((m.start, m.end, s.clone()));
                                            }
                                        }
                                    });
                                }
                            }
                            ui.horizontal(|ui| {
                                if ui.small_button(t("panel.add")).clicked() {
                                    add = Some(m.word.clone());
                                }
                                if ui.small_button(t("panel.jump")).clicked() {
                                    jump = Some((m.start, m.end));
                                }
                            });
                        });
                        ui.add_space(3.0);
                    }
                });
            });

        if close {
            self.show_spelling_panel = false;
        }
        if let Some((s, e, txt)) = replace {
            if e <= self.source.len() && self.source.is_char_boundary(s) && self.source.is_char_boundary(e) {
                self.source.replace_range(s..e, &txt);
                self.modified = true;
                self.segments_dirty = true;
            }
        } else if let Some(w) = add {
            if let Some(sc) = self.spell.as_mut() {
                sc.add_word(&w);
            }
            self.spell_sugg_cache.remove(&w);
            self.segments_dirty = true;
        } else if let Some((s, e)) = jump {
            self.selection_anchor = byte_to_char_index(&self.source, s);
            self.cursor_pos = byte_to_char_index(&self.source, e);
            if self.view_mode == ViewMode::Editor {
                self.view_mode = ViewMode::Split;
            }
            self.request_source_focus = true;
            self.apply_cursor_to_editor_state();
        }
    }

    /// Start an opt-in background download of a dictionary (wooorm/dictionaries)
    /// into `<exe-dir>/dictionaries/`. The UI polls [`poll_dict_download`].
    fn start_dict_download(&mut self, lang: &str, repo: &str) {
        if self.dict_dl.is_some() {
            self.dict_status = "A download is already running...".into();
            return;
        }
        let Some(dir) = dict_dir() else {
            self.dict_status = "Cannot locate the dictionaries folder".into();
            return;
        };
        if std::fs::create_dir_all(&dir).is_err() {
            self.dict_status = "Cannot create the dictionaries folder".into();
            return;
        }
        let state = std::sync::Arc::new(std::sync::Mutex::new(DictDownload {
            lang: lang.to_string(),
            done: false,
            error: None,
        }));
        self.dict_dl = Some(state.clone());
        self.dict_status = format!("Downloading {lang}...");
        self.status_msg = format!("Downloading {lang} dictionary...");
        let lang = lang.to_string();
        let repo = repo.to_string();
        std::thread::spawn(move || {
            let base = format!(
                "https://raw.githubusercontent.com/wooorm/dictionaries/main/dictionaries/{repo}"
            );
            // Download as raw bytes (dictionaries can be multi-MB and need no
            // UTF-8 assumption); write straight to disk.
            let fetch = |url: String| -> Result<Vec<u8>, String> {
                let resp = ureq::get(&url).call().map_err(|e| e.to_string())?;
                let mut buf = Vec::new();
                std::io::Read::read_to_end(&mut resp.into_reader(), &mut buf)
                    .map_err(|e| e.to_string())?;
                Ok(buf)
            };
            let res = (|| -> Result<(), String> {
                let dic = fetch(format!("{base}/index.dic"))?;
                let aff = fetch(format!("{base}/index.aff"))?;
                std::fs::write(dir.join(format!("{lang}.dic")), &dic).map_err(|e| e.to_string())?;
                std::fs::write(dir.join(format!("{lang}.aff")), &aff).map_err(|e| e.to_string())?;
                Ok(())
            })();
            if let Ok(mut s) = state.lock() {
                s.error = res.err();
                s.done = true;
            }
        });
    }

    /// Poll the in-flight download; on completion, load the new dictionary.
    pub(crate) fn poll_dict_download(&mut self, ctx: &egui::Context) {
        let Some(state) = self.dict_dl.clone() else { return };
        ctx.request_repaint(); // keep the loop alive while the thread runs
        let (done, lang, error) = {
            let Ok(s) = state.lock() else { return };
            (s.done, s.lang.clone(), s.error.clone())
        };
        if !done {
            return;
        }
        self.dict_dl = None;
        if let Some(e) = error {
            self.dict_status = format!("Download failed: {e}");
            self.status_msg = format!("Download failed: {e}");
            return;
        }
        if let Some(dir) = dict_dir() {
            let dic = dir.join(format!("{lang}.dic"));
            let aff = dir.join(format!("{lang}.aff"));
            if let (Ok(a), Ok(d)) =
                (std::fs::read_to_string(&aff), std::fs::read_to_string(&dic))
            {
                match mdall_core::spell::SpellChecker::from_aff_dic(&a, &d, &lang) {
                    Ok(sc) => {
                        self.spell = Some(sc);
                        self.spell_enabled = true;
                        self.show_spelling_panel = true;
                        self.spell_sugg_cache.clear();
                        self.segments_dirty = true;
                        self.dict_status = format!("Dictionary '{lang}' downloaded and active");
                        self.status_msg = format!("Dictionary '{lang}' downloaded");
                        return;
                    }
                    Err(e) => {
                        self.dict_status = format!("Downloaded '{lang}' but parse failed: {e}");
                        return;
                    }
                }
            }
        }
        self.dict_status = format!("Downloaded '{lang}' but could not read the files");
    }

    /// Auto-load the first Hunspell dictionary found in `<exe-dir>/dictionaries/`
    /// at startup (this is where downloaded/bundled dictionaries land). Silently
    /// does nothing if the folder or a valid `.dic`+`.aff` pair is absent.
    pub(crate) fn autoload_default_dictionary(&mut self) {
        let Some(dir) = std::env::current_exe()
            .ok()
            .and_then(|p| p.parent().map(|d| d.join("dictionaries")))
        else {
            return;
        };
        let Ok(entries) = std::fs::read_dir(&dir) else { return };
        for entry in entries.flatten() {
            let p = entry.path();
            if p.extension().and_then(|x| x.to_str()) != Some("dic") {
                continue;
            }
            let aff = p.with_extension("aff");
            if let (Ok(a), Ok(d)) = (std::fs::read_to_string(&aff), std::fs::read_to_string(&p)) {
                let lang = p.file_stem().and_then(|s| s.to_str()).unwrap_or("dict").to_string();
                if let Ok(sc) = mdall_core::spell::SpellChecker::from_aff_dic(&a, &d, &lang) {
                    self.spell = Some(sc);
                    self.spell_enabled = true;
                    self.show_spelling_panel = true;
                    break;
                }
            }
        }
    }
}
