//! Top formatting toolbar (view modes, formatting, paragraph style, zoom, font).
//! show_toolbar() is called from MdApp::update().

use eframe::egui;
use crate::MdApp;
use crate::theme;
use crate::ViewMode;
use crate::ui::state::LinkDialog;
use crate::ui::icons::{self, Icon};
use crate::i18n::t;

/// Classic preset palette (3 rows × 6): greys, vivid, soft.
const COLOR_PRESETS: [(u8, u8, u8); 18] = [
    (0, 0, 0), (68, 68, 68), (136, 136, 136), (170, 170, 170), (210, 210, 210), (255, 255, 255),
    (231, 76, 60), (230, 126, 34), (241, 196, 15), (39, 174, 96), (41, 128, 185), (142, 68, 173),
    (255, 182, 193), (255, 160, 122), (144, 238, 144), (22, 160, 133), (173, 216, 230), (216, 191, 216),
];

/// Color control offering BOTH a preset palette AND the HSV mixer in one popup.
/// `label` = the swatch glyph ("A" text color, "H" highlight). `highlight` tints
/// the glyph background instead of its text. Returns Some(rgb) when a color is
/// picked this frame (palette click or mixer change).
fn color_picker_combo(
    ui: &mut egui::Ui,
    id: &str,
    label: &str,
    rgb: [u8; 3],
    highlight: bool,
) -> Option<[u8; 3]> {
    let cur = egui::Color32::from_rgb(rgb[0], rgb[1], rgb[2]);
    let rich = if highlight {
        egui::RichText::new(label).size(14.0).background_color(cur)
    } else {
        egui::RichText::new(label).size(14.0).color(cur)
    };
    let resp = ui.add(egui::Button::new(rich).min_size(egui::vec2(26.0, 24.0)));
    let popup_id = egui::Id::new(("colorpop", id));
    if resp.clicked() {
        ui.memory_mut(|m| m.toggle_popup(popup_id));
    }
    let mut result = None;
    // Set when a discrete palette swatch is clicked, so the popup closes right
    // after the colour is applied. The custom mixer does NOT close it (the user
    // is dragging to fine-tune; closing on every change would fight the drag).
    let mut close_after = false;
    egui::popup::popup_below_widget(
        ui,
        popup_id,
        &resp,
        egui::PopupCloseBehavior::CloseOnClickOutside,
        |ui| {
            ui.spacing_mut().item_spacing = egui::vec2(3.0, 3.0);
            egui::Grid::new((id, "presets")).spacing([3.0, 3.0]).show(ui, |ui| {
                for (i, &(r, g, b)) in COLOR_PRESETS.iter().enumerate() {
                    let c = egui::Color32::from_rgb(r, g, b);
                    let (rect, s) = ui.allocate_exact_size(egui::vec2(16.0, 16.0), egui::Sense::click());
                    ui.painter().rect_filled(rect, 3.0, c);
                    let edge = if s.hovered() {
                        egui::Stroke::new(2.0, theme::ACCENT)
                    } else {
                        egui::Stroke::new(1.0, theme::BORDER)
                    };
                    ui.painter().rect_stroke(rect, 3.0, edge);
                    if s.clicked() {
                        result = Some([r, g, b]);
                        close_after = true;
                    }
                    if (i + 1) % 6 == 0 {
                        ui.end_row();
                    }
                }
            });
            // HSV mixer tucked into a collapsible row so the popup stays compact
            // (palette only) by default; expand it for a custom colour. The picker
            // only edits an in-progress colour (persisted across frames); it is
            // applied to the text ONLY on the explicit Apply button - which also
            // closes the popup. Applying live on every drag would splice a new
            // <span> into the source each frame (nested-markup bug).
            egui::CollapsingHeader::new(egui::RichText::new("Custom...").small().color(theme::TEXT_2))
                .id_salt((id, "mixer"))
                .default_open(false)
                .show(ui, |ui| {
                    let mem_id = egui::Id::new((id, "mixval"));
                    let mut c = ui.data(|d| d.get_temp::<egui::Color32>(mem_id)).unwrap_or(cur);
                    egui::color_picker::color_picker_color32(ui, &mut c, egui::color_picker::Alpha::Opaque);
                    ui.data_mut(|d| d.insert_temp(mem_id, c));
                    ui.add_space(2.0);
                    if ui.add(egui::Button::new(
                        egui::RichText::new("Apply").small().color(theme::text_strong(false)),
                    ).min_size(egui::vec2(ui.available_width(), 20.0)))
                        .clicked()
                    {
                        result = Some([c.r(), c.g(), c.b()]);
                        close_after = true;
                        ui.data_mut(|d| d.remove::<egui::Color32>(mem_id));
                    }
                });
        },
    );
    if close_after {
        ui.memory_mut(|m| m.close_popup());
    }
    result
}

impl MdApp {
    pub(crate) fn show_toolbar(&mut self, ctx: &egui::Context) {
        let dark = self.dark_mode;
        egui::TopBottomPanel::top("toolbar")
            .min_height(56.0) // was 70.0 - more compact
            .frame(egui::Frame::default()
                .fill(theme::panel_bg(dark))
                .inner_margin(egui::Margin { left: 6.0, right: 6.0, top: 3.0, bottom: 3.0 }))
            .show(ctx, |ui| {
            // Warm button helpers - consistent fill across toolbar
            let warm_fill = theme::btn_fill(dark); // warm grey (not cold)
            let btn = |label: egui::RichText| {
                egui::Button::new(label.color(theme::text_soft(dark)))
                    .min_size(egui::vec2(32.0, 26.0))
                    .fill(warm_fill)
            };
            let ibtn = |label: &str| btn(egui::RichText::new(label).size(14.0));

            // ── Row 1: text formatting + headings ───────────────────────────
            // (The view-mode switcher moved to the top bar - see draw_view_switcher,
            //  which is hover-revealed on the clean converter home.)
            ui.horizontal(|ui| {
                ui.spacing_mut().item_spacing.x = 3.0;

                // Character formatting - buttons light up when cursor is in matching format
                // (only active in Editor mode; in Source mode buttons always appear normal)
                let wysiwyg = self.view_mode == ViewMode::Editor;
                let fmt = self.wysiwyg_fmt;
                // ── Format button helper: explicit colors, no surprise states ──
                // Active  = logo gold bg   + black text  (4.9:1 contrast ✓)
                // Inactive = warm grey bg  + dark text   (7:1 contrast ✓)
                let fmt_btn = |label: egui::RichText, active: bool| {
                    let (bg, txt) = if active {
                        (theme::ACCENT, theme::TEXT)                // gold + dark ink
                    } else {
                        (theme::btn_fill(dark), theme::text_soft(dark)) // warm grey + mid-dark
                    };
                    egui::Button::new(label.color(txt))
                        .min_size(egui::vec2(32.0, 26.0))
                        .fill(bg)
                };

                if icons::icon_toggle(ui, Icon::Bold, wysiwyg && fmt.bold, "Bold (Ctrl+B)").clicked() { self.wrap_text("**", "**"); }
                if icons::icon_toggle(ui, Icon::Italic, wysiwyg && fmt.italic, "Italic (Ctrl+I)").clicked() { self.wrap_text("*", "*"); }
                if icons::icon_toggle(ui, Icon::Underline, false, "Underline (Ctrl+U)").clicked() { self.wrap_text("<u>", "</u>"); }
                if icons::icon_toggle(ui, Icon::Strikethrough, wysiwyg && fmt.strikethrough, "Strikethrough").clicked() { self.wrap_text("~~", "~~"); }
                if ui.add(fmt_btn(egui::RichText::new("x²").size(15.0), false))
                    .on_hover_text("Superscript").clicked() { self.wrap_text("<sup>", "</sup>"); }
                if ui.add(fmt_btn(egui::RichText::new("x₂").size(15.0), false))
                    .on_hover_text("Subscript").clicked() { self.wrap_text("<sub>", "</sub>"); }
                if icons::icon_toggle(ui, Icon::Code, wysiwyg && fmt.code, "Inline Code").clicked() { self.wrap_text("`", "`"); }
                if ui.add(fmt_btn(egui::RichText::new("ab̲").size(14.0), false))
                    .on_hover_text("Mark / Highlight").clicked() { self.wrap_text("<mark>", "</mark>"); }

                ui.separator();

                // ── Paragraph style - ComboBox Word-style (Normal / H1-H6) ────
                {
                    let current_style = if wysiwyg && fmt.heading > 0 {
                        match fmt.heading {
                            1 => "Heading 1", 2 => "Heading 2", 3 => "Heading 3",
                            4 => "Heading 4", 5 => "Heading 5", _ => "Heading 6",
                        }
                    } else { "Normal" };

                    let sel_txt = egui::RichText::new(current_style).size(12.0).color(theme::text_soft(dark));
                    egui::ComboBox::from_id_salt("para_style")
                        .width(90.0)
                        .selected_text(sel_txt)
                        .show_ui(ui, |ui| {
                            let _ = ui.selectable_label(current_style == "Normal",
                                egui::RichText::new("Normal").size(13.0).color(theme::text_soft(dark)));
                            ui.separator();
                            for (label, prefix, lvl, sz) in [
                                ("Heading 1", "# ",   1u8, 16.0f32),
                                ("Heading 2", "## ",  2,   14.5),
                                ("Heading 3", "### ", 3,   13.5),
                                ("Heading 4", "#### ",4,   12.5),
                                ("Heading 5", "##### ",5,   12.0),
                                ("Heading 6", "###### ",6,  11.5),
                            ] {
                                if ui.selectable_label(
                                    wysiwyg && fmt.heading == lvl,
                                    egui::RichText::new(label).size(sz).strong().color(theme::text_strong(dark)),
                                ).clicked() {
                                    self.insert_text(prefix);
                                }
                            }
                        });
                }

                ui.separator();

                // Text color - preset palette OR custom mixer
                if let Some(rgb) = color_picker_combo(ui, "textcol", "A", self.text_color, false) {
                    self.text_color = rgb;
                    let hex = format!("{:02x}{:02x}{:02x}", rgb[0], rgb[1], rgb[2]);
                    self.wrap_text(&format!("<span style=\"color:#{}\">", hex), "</span>");
                }
                // Highlight color - preset palette OR custom mixer
                if let Some(rgb) = color_picker_combo(ui, "hlcol", "H", self.highlight_color, true) {
                    self.highlight_color = rgb;
                    let hex = format!("{:02x}{:02x}{:02x}", rgb[0], rgb[1], rgb[2]);
                    self.wrap_text(&format!("<mark style=\"background:#{}\">", hex), "</mark>");
                }

                // ── Right group: theme toggle + settings (light is the default) ──
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if icons::icon_button(ui, Icon::Settings, "Options").clicked() {
                        self.options_open = true;
                    }
                    let (theme_icon, theme_tip) = if self.dark_mode {
                        (Icon::Sun, "Switch to light theme")
                    } else {
                        (Icon::Moon, "Switch to dark theme")
                    };
                    if icons::icon_button(ui, theme_icon, theme_tip).clicked() {
                        self.dark_mode = !self.dark_mode;
                    }
                });
            });

            // ── Row 2: alignment + lists + insert + zoom + font ──────────────
            ui.horizontal(|ui| {
                ui.spacing_mut().item_spacing.x = 3.0;

                // Paragraph alignment - labelled, with bigger icons
                if icons::icon_button(ui, Icon::AlignLeft, "Align Left").clicked() { self.wrap_block_align("left"); }
                if icons::icon_button(ui, Icon::AlignCenter, "Center").clicked() { self.wrap_block_align("center"); }
                if icons::icon_button(ui, Icon::AlignRight, "Align Right").clicked() { self.wrap_block_align("right"); }
                if icons::icon_button(ui, Icon::AlignJustify, "Justify").clicked() { self.wrap_block_align("justify"); }

                ui.separator();

                // Lists & structure
                if icons::icon_button(ui, Icon::ListBullet, "Bullet List").clicked() { self.insert_text("- "); }
                if icons::icon_button(ui, Icon::ListNumber, "Numbered List").clicked() { self.insert_text("1. "); }
                if icons::icon_button(ui, Icon::Quote, "Blockquote").clicked() { self.insert_text("> "); }
                if icons::icon_button(ui, Icon::Code, "Code Block").clicked() { self.insert_text("```\n\n```\n"); }
                if icons::icon_button(ui, Icon::Rule, "Horizontal Rule").clicked() { self.insert_text("---\n"); }
                if icons::icon_button(ui, Icon::Table, "Insert Table").clicked() {
                    self.insert_text("| Col 1 | Col 2 | Col 3 |\n|--------|--------|--------|\n|  |  |  |\n");
                }

                ui.separator();

                // Equations - gold to match brand identity
                if ui.add(btn(egui::RichText::new("∑").size(18.0).color(theme::ACCENT)))
                    .on_hover_text("Equation Block (Ctrl+E)").clicked() {
                    self.insert_text("$$\n\\sum_{i=0}^{n} x_i\n$$\n");
                }
                if ui.add(btn(egui::RichText::new("∑ᵢ").size(14.0).color(theme::ACCENT_HOVER)))
                    .on_hover_text("Inline Equation ($)").clicked() {
                    self.wrap_text("$", "$");
                }

                ui.separator();

                // Links / images / search
                if icons::icon_button(ui, Icon::Link, "Insert Link (Ctrl+K)").clicked() {
                    self.open_link_dialog(false);
                }
                if icons::icon_button(ui, Icon::Image, "Insert Image").clicked() {
                    self.open_link_dialog(true);
                }
                if icons::icon_button(ui, Icon::Search, "Find & Replace (Ctrl+H)").clicked() {
                    self.show_search = !self.show_search;
                }

                ui.separator();

                // ── Zoom - boutons +/- + ComboBox presets ────────────────────
                if ui.add(ibtn("−")).on_hover_text("Zoom Out (Ctrl+-)").clicked() {
                    self.zoom_level = (self.zoom_level - 0.1).max(0.3);
                }
                {
                    let zoom_pct = (self.zoom_level * 100.0).round() as u32;
                    let zoom_sel = egui::RichText::new(format!("{}%", zoom_pct)).size(12.0).color(theme::text_soft(dark));
                    egui::ComboBox::from_id_salt("zoom_pick")
                        .width(56.0)
                        .selected_text(zoom_sel)
                        .show_ui(ui, |ui| {
                            for (pct, val) in [(50u32,0.5f32),(75,0.75),(100,1.0),(125,1.25),(150,1.5),(175,1.75),(200,2.0),(250,2.5),(300,3.0)] {
                                if ui.selectable_label(
                                    zoom_pct == pct,
                                    egui::RichText::new(format!("{}%", pct)).size(12.5).color(theme::text_soft(dark)),
                                ).clicked() {
                                    self.zoom_level = val;
                                }
                            }
                        });
                }
                if ui.add(ibtn("+")).on_hover_text("Zoom In (Ctrl+=)").clicked() {
                    self.zoom_level = (self.zoom_level + 0.1).min(3.0);
                }

                ui.separator();

                // Font
                let prev_font = self.selected_font.clone();
                egui::ComboBox::from_id_salt("font_sel")
                    .width(130.0)
                    .selected_text(egui::RichText::new(&self.selected_font).size(13.0))
                    .show_ui(ui, |ui| {
                        for (name, _path) in &self.font_list {
                            if name == "---" {
                                ui.separator();
                            } else {
                                ui.selectable_value(&mut self.selected_font, name.clone(),
                                    egui::RichText::new(name.as_str()).size(13.0));
                            }
                        }
                    });
                if self.selected_font != prev_font { self.apply_font_change(ctx); }

                // ── Taille de police - ComboBox (clic) + DragValue (précision) ──
                {
                    let std_sizes = [8.0f32, 9.0, 10.0, 11.0, 12.0, 14.0, 16.0, 18.0,
                                     20.0, 22.0, 24.0, 26.0, 28.0, 36.0, 48.0, 72.0];
                    let sz_label = egui::RichText::new(format!("{}", self.font_size as u32))
                        .size(12.0).color(theme::text_soft(dark));
                    let prev_fs = self.font_size;
                    egui::ComboBox::from_id_salt("font_size_pick")
                        .width(44.0)
                        .selected_text(sz_label)
                        .show_ui(ui, |ui| {
                            for &sz in &std_sizes {
                                if ui.selectable_label(
                                    (self.font_size - sz).abs() < 0.5,
                                    egui::RichText::new(format!("{}", sz as u32)).size(12.5).color(theme::text_soft(dark)),
                                ).clicked() {
                                    self.font_size = sz;
                                }
                            }
                        });
                    // DragValue conservé pour la précision / le drag / la saisie directe
                    ui.add(
                        egui::DragValue::new(&mut self.font_size)
                            .range(6.0..=96.0)
                            .speed(0.5)
                            .max_decimals(1)
                    ).on_hover_text("Drag ou saisie directe");
                    ui.label(egui::RichText::new("pt").size(11.0).color(theme::text_faint(dark)));
                    if (self.font_size - prev_fs).abs() > 0.1 { self.segments_dirty = true; }
                }
            });

            // ── Gold identity stripe at bottom of toolbar ────────────────────
            // This 2px line appears in every mode (Hub / Source / Split / Editor)
            // and anchors the brand identity throughout the application.
            let r = ui.max_rect();
            ui.painter().line_segment(
                [egui::pos2(r.left(), r.bottom() - 1.0), egui::pos2(r.right(), r.bottom() - 1.0)],
                egui::Stroke::new(2.0, theme::ACCENT),
            );
        });
    }
}

impl MdApp {
    pub(crate) fn show_search_bar(&mut self, ctx: &egui::Context) {
        if self.show_search {
            egui::TopBottomPanel::top("searchbar").show(ctx, |ui| {
                ui.add_space(4.0);
                let prev_query = self.search_query.clone();

                // ── Row 1: Find ───────────────────────────────────────────
                ui.horizontal(|ui| {
                    let (sr, _) = ui.allocate_exact_size(egui::vec2(22.0, 26.0), egui::Sense::hover());
                    icons::paint_icon(ui.painter(), Icon::Search, sr.shrink(5.0),
                        ui.visuals().widgets.inactive.fg_stroke.color);

                    // Multiline search field - grows with content, max ~4 lines visible.
                    // Enter inserts a newline (for multiline LaTeX blocks).
                    // Ctrl+Enter = find next.  Shift+Ctrl+Enter = find prev.
                    let search_resp = egui::ScrollArea::vertical()
                        .id_salt("search_scroll")
                        .max_height(80.0)
                        .show(ui, |ui| {
                            ui.add(
                                egui::TextEdit::multiline(&mut self.search_query)
                                    .desired_width(240.0)
                                    .desired_rows(1)
                                    .hint_text("Find...  (Enter = newline, Ctrl+Enter = search)"),
                            )
                        })
                        .inner;

                    // Recompute whenever query changes
                    if self.search_query != prev_query {
                        self.search_match_idx = 0;
                        self.compute_search_matches();
                    }

                    // Ctrl+Enter → find next/prev (Enter is kept for newlines)
                    if search_resp.has_focus()
                        && ui.input(|i| i.modifiers.ctrl && i.key_pressed(egui::Key::Enter))
                    {
                        if ui.input(|i| i.modifiers.shift) {
                            self.do_find_prev();
                        } else {
                            self.do_find_next();
                        }
                    }

                    // Match counter
                    let count = self.search_matches.len();
                    if !self.search_query.is_empty() {
                        let label = if count == 0 {
                            egui::RichText::new("No match")
                                .color(egui::Color32::from_rgb(200, 60, 60))
                                .size(12.0)
                        } else {
                            egui::RichText::new(format!("{}/{}", self.search_match_idx + 1, count))
                                .color(egui::Color32::GRAY)
                                .size(12.0)
                        };
                        ui.label(label);
                    }

                    // Prev / Next
                    if ui.add_sized([24.0, 26.0], egui::Button::new("◀"))
                        .on_hover_text("Previous match  Shift+F3").clicked()
                    {
                        self.do_find_prev();
                    }
                    if ui.add_sized([24.0, 26.0], egui::Button::new("▶"))
                        .on_hover_text("Next match  F3  or  Ctrl+Enter").clicked()
                    {
                        self.do_find_next();
                    }

                    // Case-sensitive toggle
                    let aa_color = if self.search_case_sensitive {
                        egui::Color32::from_rgb(60, 120, 220)
                    } else {
                        egui::Color32::GRAY
                    };
                    if ui.add_sized(
                        [28.0, 26.0],
                        egui::SelectableLabel::new(
                            self.search_case_sensitive,
                            egui::RichText::new("Aa").color(aa_color).size(12.0),
                        ),
                    )
                    .on_hover_text("Case sensitive")
                    .clicked()
                    {
                        self.search_case_sensitive = !self.search_case_sensitive;
                        self.search_match_idx = 0;
                        self.compute_search_matches();
                    }

                    // Toggle Replace row
                    let replace_icon = if self.search_show_replace { "⊟" } else { "⊞" };
                    if ui.add_sized([24.0, 26.0], egui::Button::new(replace_icon))
                        .on_hover_text(if self.search_show_replace {
                            "Hide replace"
                        } else {
                            "Show replace  Ctrl+H"
                        })
                        .clicked()
                    {
                        self.search_show_replace = !self.search_show_replace;
                    }

                    // Close
                    if icons::icon_button(ui, Icon::Close, "Close  Escape").clicked() {
                        self.show_search = false;
                    }
                });

                // ── Row 2: Replace (Ctrl+H mode only) ────────────────────
                if self.search_show_replace {
                    ui.add_space(2.0);
                    ui.horizontal(|ui| {
                        ui.label(
                            egui::RichText::new("⟳")
                                .size(14.0)
                                .color(egui::Color32::GRAY),
                        );

                        // Multiline replace field - supports full LaTeX block replacement
                        egui::ScrollArea::vertical()
                            .id_salt("replace_scroll")
                            .max_height(80.0)
                            .show(ui, |ui| {
                                ui.add(
                                    egui::TextEdit::multiline(&mut self.replace_query)
                                        .desired_width(240.0)
                                        .desired_rows(1)
                                        .hint_text("Replace with...  (multiline supported)"),
                                );
                            });

                        let has_match = !self.search_matches.is_empty();
                        ui.vertical(|ui| {
                            ui.add_space(4.0);
                            if ui.add_enabled(
                                has_match,
                                egui::Button::new("Replace").min_size(egui::vec2(70.0, 24.0)),
                            )
                            .on_hover_text("Replace this occurrence  Ctrl+R")
                            .clicked()
                            {
                                self.do_replace_current();
                            }
                            if ui.add_enabled(
                                has_match,
                                egui::Button::new("Replace All").min_size(egui::vec2(70.0, 24.0)),
                            )
                            .on_hover_text("Replace all occurrences")
                            .clicked()
                            {
                                self.do_replace_all();
                            }
                        });
                    });
                }
                ui.add_space(4.0);
            });
        }
    }
}

impl MdApp {
    fn draw_view_switcher(&mut self, ui: &mut egui::Ui, dark: bool) {
        let view_btn = |label: &str, active: bool| {
            let (bg, txt) = if active {
                (theme::ACCENT, theme::TEXT)
            } else {
                (theme::btn_fill(dark), theme::text_soft(dark))
            };
            egui::Button::new(egui::RichText::new(label).size(12.0).color(txt)).fill(bg)
        };
        if ui.add_sized([44.0, 22.0], view_btn(&t("view.hub"), self.view_mode == ViewMode::Converter)).clicked() { self.view_mode = ViewMode::Converter; }
        if ui.add_sized([54.0, 22.0], view_btn(&t("view.source"), self.view_mode == ViewMode::Source)).clicked() { self.view_mode = ViewMode::Source; }
        if ui.add_sized([42.0, 22.0], view_btn(&t("view.split"), self.view_mode == ViewMode::Split)).clicked() { self.view_mode = ViewMode::Split; }
        if ui.add_sized([54.0, 22.0], view_btn(&t("view.editor"), self.view_mode == ViewMode::Editor)).clicked() { self.view_mode = ViewMode::Editor; self.segments_dirty = true; }
    }

    /// Top bar. On the converter home it stays hidden as a thin hint strip and
    /// only reveals the view switcher + menus when `revealed` (pointer near the
    /// top edge or a menu is open). In editor modes it is always the full bar.
    pub(crate) fn show_menu_bar(&mut self, ctx: &egui::Context, home: bool, revealed: bool) {
        let dark = self.dark_mode;
        if home && !revealed {
            egui::TopBottomPanel::top("menubar")
                .exact_height(26.0)
                .frame(egui::Frame::default()
                    .fill(theme::panel_bg(dark))
                    .inner_margin(egui::Margin::symmetric(12.0, 4.0)))
                .show(ctx, |ui| {
                    ui.horizontal(|ui| {
                        ui.label(egui::RichText::new("\u{2261}  Menu").size(12.5).color(theme::text_soft(dark)));
                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            ui.label(egui::RichText::new("\u{25BE}  hover for menus").size(11.0).color(theme::TEXT_MUTED));
                        });
                    });
                });
            return;
        }
        egui::TopBottomPanel::top("menubar").show(ctx, |ui| {
            egui::menu::bar(ui, |ui| {
                self.draw_view_switcher(ui, dark);
                ui.separator();
                ui.menu_button(t("menu.file"), |ui| {
                    if ui.button("New          Ctrl+N").clicked() { self.do_new(); ui.close_menu(); }
                    if ui.button("Open         Ctrl+O").clicked() { self.do_open(); ui.close_menu(); }
                    if ui.button("Save         Ctrl+S").clicked() { self.do_save(); ui.close_menu(); }
                    if ui.button("Save As...   Ctrl+Shift+S").clicked() { self.do_save_as(); ui.close_menu(); }
                    ui.separator();
                    if ui.button("\u{21BA} Import DOCX...").on_hover_text(
                        "Re-import a .docx exported by MD -> ALL.\n\
                         Recovers original markdown + LaTeX intact."
                    ).clicked() {
                        self.do_import_docx();
                        ui.close_menu();
                    }
                    ui.separator();
                    if ui.button("Export As...").clicked() { self.export_dialog.visible = true; ui.close_menu(); }
                    if ui.button("Quick PDF").clicked() { self.do_export_pdf(); ui.close_menu(); }
                    if ui.button("Quick HTML").clicked() { self.do_export_html(); ui.close_menu(); }
                    ui.separator();
                    if ui.button("Print        Ctrl+P").clicked() { self.do_print(); ui.close_menu(); }
                    ui.separator();
                    if ui.button("Exit").clicked() { ctx.send_viewport_cmd(egui::ViewportCommand::Close); }
                });
                ui.menu_button(t("menu.edit"), |ui| {
                    if ui.button("Find & Replace   Ctrl+H").clicked() { self.show_search = !self.show_search; ui.close_menu(); }
                });
                ui.menu_button(t("menu.insert"), |ui| {
                    if ui.button("Heading 1").clicked() { self.insert_text("# "); ui.close_menu(); }
                    if ui.button("Heading 2").clicked() { self.insert_text("## "); ui.close_menu(); }
                    if ui.button("Heading 3").clicked() { self.insert_text("### "); ui.close_menu(); }
                    ui.separator();
                    if ui.button("Bold              Ctrl+B").clicked() { self.wrap_text("**", "**"); ui.close_menu(); }
                    if ui.button("Italic            Ctrl+I").clicked() { self.wrap_text("*", "*"); ui.close_menu(); }
                    if ui.button("Underline         Ctrl+U").clicked() { self.wrap_text("<u>", "</u>"); ui.close_menu(); }
                    if ui.button("Strikethrough").clicked() { self.wrap_text("~~", "~~"); ui.close_menu(); }
                    if ui.button("Inline Code").clicked() { self.wrap_text("`", "`"); ui.close_menu(); }
                    ui.separator();
                    if ui.button("Code Block").clicked() { self.insert_text("```\n\n```\n"); ui.close_menu(); }
                    if ui.button("Equation Block    Ctrl+E").clicked() { self.insert_text("$$\n\\sum_{i=0}^{n} x_i\n$$\n"); ui.close_menu(); }
                    if ui.button("Inline Equation").clicked() { self.wrap_text("$", "$"); ui.close_menu(); }
                    ui.separator();
                    if ui.button("Link...           Ctrl+K").clicked() {
                        self.link_dialog = LinkDialog { visible: true, text: String::new(), url: String::new(), is_image: false };
                        ui.close_menu();
                    }
                    if ui.button("Image...").clicked() {
                        self.link_dialog = LinkDialog { visible: true, text: String::new(), url: String::new(), is_image: true };
                        ui.close_menu();
                    }
                    if ui.button("Image from File...").clicked() { self.do_insert_image_file(); ui.close_menu(); }
                    ui.separator();
                    if ui.button("Table").clicked() { self.insert_text("| Col 1 | Col 2 | Col 3 |\n|--------|--------|--------|\n|  |  |  |\n"); ui.close_menu(); }
                    if ui.button("List Item").clicked() { self.insert_text("- "); ui.close_menu(); }
                    if ui.button("Blockquote").clicked() { self.insert_text("> "); ui.close_menu(); }
                    if ui.button("Horizontal Rule").clicked() { self.insert_text("---\n"); ui.close_menu(); }
                });
                if ui.button(t("menu.metadata")).clicked() { self.show_metadata = true; }
                if ui.button(t("menu.modules")).clicked() { self.module_open = true; }
            });
        });
    }

    pub(crate) fn show_status_bar(&mut self, ctx: &egui::Context) {
        let dark = self.dark_mode;
        egui::TopBottomPanel::bottom("statusbar")
            .frame(egui::Frame::default()
                .fill(theme::surface_soft_c(dark))
                .inner_margin(egui::Margin::symmetric(8.0, 3.0)))
            .show(ctx, |ui| {
            ui.horizontal(|ui| {
                let muted = theme::text_faint(dark);
                let dot = if self.modified {
                    egui::RichText::new("\u{25CF} ").color(theme::ACCENT).size(11.5)
                } else {
                    egui::RichText::new("").size(11.5)
                };
                ui.label(dot);
                ui.label(egui::RichText::new(&self.status_msg).size(11.5).color(theme::text_soft(dark)));
                ui.separator();
                if let Some(ref p) = self.current_file {
                    ui.label(egui::RichText::new(p.display().to_string()).size(11.5).color(muted));
                } else {
                    ui.label(egui::RichText::new("Untitled").size(11.5).color(muted));
                }
                // Reviewer-feedback toggle (only when an imported DOCX carried any).
                if !self.review_items.is_empty() {
                    ui.separator();
                    let on = self.show_review_panel;
                    if ui.selectable_label(on, egui::RichText::new(
                        format!("\u{1F4DD} {} ({})", t("panel.review"), self.review_items.len())
                    ).size(11.5)).clicked() {
                        self.show_review_panel = !on;
                    }
                }
                // Spelling toggle (only when a dictionary is loaded + enabled).
                if self.spell_enabled && self.spell.is_some() {
                    ui.separator();
                    let on = self.show_spelling_panel;
                    if ui.selectable_label(on, egui::RichText::new(
                        format!("\u{2713} {} ({})", t("panel.spelling"), self.spell_issues.len())
                    ).size(11.5)).clicked() {
                        self.show_spelling_panel = !on;
                    }
                }
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    // cursor_pos is a CHAR index - count newlines among the first N
                    // chars (slicing source[..cursor_pos] as bytes panics on multibyte
                    // text, e.g. an accented title like "théorème").
                    let lines = self.source.chars().take(self.cursor_pos)
                        .filter(|&c| c == '\n').count() + 1;
                    let char_count = self.source.chars().count();
                    let words = mdall_core::stats::word_count(&self.source);
                    ui.label(egui::RichText::new(
                        format!("Ln {} | {} words | {} chars | {}%",
                            lines, words, char_count, (self.zoom_level * 100.0) as u32)
                    ).size(11.5).color(muted));
                    ui.separator();
                    ui.label(egui::RichText::new(match self.view_mode {
                        ViewMode::Converter => t("view.hub"),
                        ViewMode::Source    => t("view.source"),
                        ViewMode::Split     => t("view.split"),
                        ViewMode::Editor    => t("view.editor"),
                    }).size(11.5).color(muted));
                });
            });
        });
    }
}
