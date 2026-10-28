//! The Export window: Image, Animation and Sprite Sheet tabs.
//!
//! The dialog only collects choices, previews them and starts an [`ExportJob`]. All
//! rendering and encoding lives in [`crate::export`]; the document is only read when a
//! snapshot is taken.

use std::path::PathBuf;
use std::sync::Arc;
use std::time::Instant;

use egui::{Color32, ColorImage, RichText, TextureHandle, TextureOptions, Vec2};

use crate::core::Rgba;
use crate::editor::Editor;
use crate::export::render::{scaled_size, snapshot, FrameImage};
use crate::export::service::{ExportJob, ExportRequest, ExportTarget};
use crate::export::{
    check_output_size, gif, sequence, spritesheet, still, ExportKind, ExportSettings,
    ExportSummary, FrameSelection, GifLoop, LayerSource, MetadataFormat, SheetLayout, StillFormat,
    MAX_EXTRUDE, MAX_SCALE, MAX_SPACING,
};
use crate::services::dialogs as native;
use crate::theme;
use crate::ui::dialogs::accent_button;

/// Largest preview size in points.
const PREVIEW: Vec2 = Vec2::new(300.0, 210.0);

struct Preview {
    key: (Vec<usize>, LayerSource, u64),
    frames: Vec<FrameImage>,
    textures: Vec<TextureHandle>,
    sheet: Option<(String, TextureHandle)>,
    started: Instant,
}

/// What the app should do after the dialog ran this frame.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ExportDialogAction {
    None,
    /// Settings changed: save them soon.
    SettingsChanged,
}

#[derive(Default)]
pub struct ExportDialog {
    pub open: bool,
    job: Option<ExportJob>,
    result: Option<Result<ExportSummary, String>>,
    /// A request waiting for "Replace existing files?".
    confirm: Option<(ExportRequest, usize)>,
    preview: Option<Preview>,
    message: Option<String>,
}

fn frames_for(selection: FrameSelection, editor: &Editor, selected: &[usize]) -> Vec<usize> {
    let doc = editor.document();
    match selection {
        FrameSelection::Current => vec![doc.active_frame_index()],
        FrameSelection::All => (0..doc.frame_count()).collect(),
        FrameSelection::Selected if selected.is_empty() => vec![doc.active_frame_index()],
        FrameSelection::Selected => selected.to_vec(),
    }
}

fn doc_revision(editor: &Editor) -> u64 {
    let doc = editor.document();
    doc.frames()
        .iter()
        .fold(doc.structure_revision(), |acc, f| {
            acc.wrapping_mul(31)
                .wrapping_add(f.revision())
                .wrapping_add(f.duration_ms().into())
        })
        .wrapping_add(doc.layer_count() as u64)
}

fn texture(ctx: &egui::Context, name: &str, img: &image::RgbaImage) -> TextureHandle {
    let ci = ColorImage::from_rgba_unmultiplied(
        [img.width() as usize, img.height() as usize],
        img.as_raw(),
    );
    ctx.load_texture(name, ci, TextureOptions::NEAREST)
}

impl ExportDialog {
    pub fn open_at(&mut self, kind: ExportKind, settings: &mut ExportSettings) {
        settings.kind = kind;
        self.open = true;
        self.result = None;
        self.confirm = None;
        self.message = None;
    }

    pub fn is_running(&self) -> bool {
        self.job.is_some()
    }

    /// Draws the dialog. `selected` are frame indices selected in the timeline.
    pub fn show(
        &mut self,
        ctx: &egui::Context,
        editor: &Editor,
        selected: &[usize],
        settings: &mut ExportSettings,
    ) -> ExportDialogAction {
        if !self.open {
            return ExportDialogAction::None;
        }
        self.poll_job(settings.open_folder_after);
        let before = settings.clone();
        let mut open = true;
        egui::Window::new(RichText::new("Export").strong())
            .id(egui::Id::new("kagero-export"))
            .open(&mut open)
            .collapsible(false)
            .resizable(false)
            .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
            .default_width(700.0)
            .show(ctx, |ui| {
                ui.set_width(700.0);
                self.contents(ui, editor, selected, settings);
            });
        if !open && !self.is_running() {
            self.open = false;
        }
        if *settings != before {
            ExportDialogAction::SettingsChanged
        } else {
            ExportDialogAction::None
        }
    }

    fn poll_job(&mut self, open_folder_after: bool) {
        let Some(job) = &self.job else {
            return;
        };
        if let Some(result) = job.state().finished {
            if let Ok(summary) = &result {
                let folder = summary.files.first().and_then(|f| f.parent());
                if let (true, Some(folder)) = (open_folder_after, folder) {
                    open_folder(folder);
                }
            }
            self.result = Some(result);
            self.job = None;
        }
    }

    fn contents(
        &mut self,
        ui: &mut egui::Ui,
        editor: &Editor,
        selected: &[usize],
        s: &mut ExportSettings,
    ) {
        let doc = editor.document();
        ui.horizontal(|ui| {
            ui.label(
                RichText::new(editor.name())
                    .strong()
                    .color(theme::TEXT_PRIMARY),
            );
            ui.label(
                RichText::new(format!(
                    "{} × {} px · {} frame(s)",
                    doc.width(),
                    doc.height(),
                    doc.frame_count()
                ))
                .small()
                .color(theme::TEXT_SECONDARY),
            );
        });
        ui.add_space(4.0);
        let running = self.is_running();
        ui.add_enabled_ui(!running, |ui| {
            ui.horizontal(|ui| {
                for (kind, label) in [
                    (ExportKind::Image, "Image"),
                    (ExportKind::Animation, "Animation"),
                    (ExportKind::SpriteSheet, "Sprite Sheet"),
                ] {
                    let selected_tab = s.kind == kind;
                    let text = RichText::new(label).strong().color(if selected_tab {
                        theme::ACCENT
                    } else {
                        theme::TEXT_PRIMARY
                    });
                    if ui
                        .add_sized(
                            [110.0, 28.0],
                            egui::SelectableLabel::new(selected_tab, text),
                        )
                        .clicked()
                    {
                        s.kind = kind;
                        self.result = None;
                        self.confirm = None;
                    }
                }
            });
        });
        ui.separator();

        // Which frames, and their snapshot for previews and sizes.
        if s.kind == ExportKind::Animation && s.frames == FrameSelection::Current {
            s.frames = FrameSelection::All;
        }
        let frames = frames_for(s.frames, editor, selected);
        self.refresh_preview(ui.ctx(), editor, &frames, s.layers);

        ui.horizontal_top(|ui| {
            ui.vertical(|ui| {
                ui.set_width(370.0);
                ui.add_enabled_ui(!running, |ui| {
                    self.options(ui, editor, selected, s, &frames);
                });
            });
            ui.separator();
            ui.vertical(|ui| {
                ui.set_width(PREVIEW.x);
                self.preview_ui(ui, s);
            });
        });
        ui.separator();
        self.footer(ui, editor, s, &frames);
    }

    fn refresh_preview(
        &mut self,
        ctx: &egui::Context,
        editor: &Editor,
        frames: &[usize],
        layers: LayerSource,
    ) {
        let key = (frames.to_vec(), layers, doc_revision(editor));
        if self.preview.as_ref().is_some_and(|p| p.key == key) {
            return;
        }
        match snapshot(editor.document(), frames, layers) {
            Ok(images) => {
                let textures = images
                    .iter()
                    .map(|f| texture(ctx, &format!("export-preview-{}", f.index), &f.image))
                    .collect();
                self.preview = Some(Preview {
                    key,
                    frames: images,
                    textures,
                    sheet: None,
                    started: Instant::now(),
                });
            }
            Err(e) => {
                self.preview = None;
                self.message = Some(e.to_string());
            }
        }
    }

    fn options(
        &mut self,
        ui: &mut egui::Ui,
        editor: &Editor,
        selected: &[usize],
        s: &mut ExportSettings,
        frames: &[usize],
    ) {
        let doc = editor.document();
        section(ui, "Format");
        match s.kind {
            ExportKind::Image => {
                ui.horizontal(|ui| {
                    for f in StillFormat::ALL {
                        ui.radio_value(&mut s.still_format, f, f.label());
                    }
                });
            }
            ExportKind::Animation => {
                ui.horizontal(|ui| {
                    let _ = ui.radio(true, "GIF");
                    ui.add_enabled(false, egui::RadioButton::new(false, "Animated WebP"))
                        .on_disabled_hover_text("Animated WebP is not available in this version.");
                });
            }
            ExportKind::SpriteSheet => {
                ui.horizontal(|ui| {
                    ui.radio_value(&mut s.sheet.image_format, StillFormat::Png, "PNG");
                    ui.radio_value(
                        &mut s.sheet.image_format,
                        StillFormat::WebP,
                        "WebP (lossless)",
                    );
                });
            }
        }

        section(ui, "Frames");
        ui.horizontal(|ui| {
            let choices: &[FrameSelection] = if s.kind == ExportKind::Animation {
                &[FrameSelection::All, FrameSelection::Selected]
            } else {
                &[
                    FrameSelection::Current,
                    FrameSelection::Selected,
                    FrameSelection::All,
                ]
            };
            for &f in choices {
                let label = match f {
                    FrameSelection::Selected => format!("Selected ({})", selected.len()),
                    FrameSelection::All => format!("All ({})", doc.frame_count()),
                    FrameSelection::Current => "Current".to_owned(),
                };
                ui.radio_value(&mut s.frames, f, label);
            }
        });
        if s.frames == FrameSelection::Selected && selected.is_empty() {
            hint(
                ui,
                "No frames selected in the timeline (Ctrl+click cards): the current frame is used.",
            );
        }
        ui.horizontal(|ui| {
            ui.label("Layers");
            egui::ComboBox::from_id_salt("export-layers")
                .selected_text(s.layers.label())
                .show_ui(ui, |ui| {
                    for l in [LayerSource::Visible, LayerSource::ActiveLayer] {
                        ui.selectable_value(&mut s.layers, l, l.label());
                    }
                });
        });

        section(ui, "Scale");
        let scale = s.scale_mut();
        ui.horizontal(|ui| {
            for v in [1u32, 2, 3, 4, 8] {
                if ui
                    .selectable_label(*scale == v, format!("{}%", v * 100))
                    .clicked()
                {
                    *scale = v;
                }
            }
            ui.add(egui::DragValue::new(scale).range(1..=MAX_SCALE).prefix("×"))
                .on_hover_text(
                    "Any whole number. Pixels are enlarged as sharp blocks (nearest neighbor).",
                );
        });
        match s.kind {
            ExportKind::Image => self.image_options(ui, s),
            ExportKind::Animation => self.gif_options(ui, s),
            ExportKind::SpriteSheet => self.sheet_options(ui, s, frames.len()),
        }

        section(ui, "File names");
        ui.horizontal(|ui| {
            ui.label("Base name");
            ui.add(
                egui::TextEdit::singleline(&mut s.naming.base_name)
                    .hint_text(sequence::sanitize_stem(editor.name()))
                    .desired_width(160.0)
                    .char_limit(100),
            );
            if s.kind == ExportKind::Image {
                ui.label("Digits");
                ui.add(egui::DragValue::new(&mut s.naming.digits).range(1..=8));
            }
        });
        if s.kind == ExportKind::Image {
            ui.checkbox(
                &mut s.naming.number_single,
                "Number the file even when exporting one frame",
            );
        }
    }

    fn image_options(&mut self, ui: &mut egui::Ui, s: &mut ExportSettings) {
        match s.still_format {
            StillFormat::Png => hint(ui, "Lossless with full transparency — best for game sprites."),
            StillFormat::WebP => hint(
                ui,
                "Lossless WebP with transparency. Lossy quality and animated WebP aren't available in this version.",
            ),
            StillFormat::Jpeg => {
                section(ui, "JPEG");
                ui.horizontal(|ui| {
                    ui.label("Quality");
                    ui.add(egui::Slider::new(&mut s.jpeg.quality, 1..=100));
                });
                ui.horizontal(|ui| {
                    ui.label("Background");
                    for (c, name) in [(Rgba::WHITE, "White"), (Rgba::BLACK, "Black")] {
                        if ui.selectable_label(s.jpeg.background == c, name).clicked() {
                            s.jpeg.background = c;
                        }
                    }
                    let mut rgb = [s.jpeg.background.r(), s.jpeg.background.g(), s.jpeg.background.b()];
                    if ui.color_edit_button_srgb(&mut rgb).on_hover_text("Custom color").changed() {
                        s.jpeg.background = Rgba::opaque(rgb[0], rgb[1], rgb[2]);
                    }
                });
                warn(ui, "JPEG has no transparency: transparent pixels are filled with the background color.");
                hint(ui, still::JPEG_WARNING);
            }
        }
    }

    fn gif_options(&mut self, ui: &mut egui::Ui, s: &mut ExportSettings) {
        section(ui, "GIF");
        ui.horizontal(|ui| {
            ui.label("Repeat");
            ui.radio_value(&mut s.gif.looping, GifLoop::Forever, "Loop forever");
            ui.radio_value(&mut s.gif.looping, GifLoop::Once, "Play once");
        });
        ui.horizontal(|ui| {
            ui.label("Transparent below alpha");
            ui.add(egui::Slider::new(&mut s.gif.alpha_threshold, 1..=255));
        })
        .response
        .on_hover_text("GIF pixels are either fully transparent or fully opaque.");
        ui.horizontal(|ui| {
            ui.label("Soft edges");
            let mut blend = s.gif.matte.is_some();
            ui.radio_value(&mut blend, false, "Keep color");
            ui.radio_value(&mut blend, true, "Blend over");
            if blend {
                let m = s.gif.matte.unwrap_or(Rgba::WHITE);
                let mut rgb = [m.r(), m.g(), m.b()];
                ui.color_edit_button_srgb(&mut rgb);
                s.gif.matte = Some(Rgba::opaque(rgb[0], rgb[1], rgb[2]));
            } else {
                s.gif.matte = None;
            }
        });
        ui.checkbox(
            &mut s.gif.dither,
            "Dither when colors must be reduced (off keeps pixel art clean)",
        );
        if let Some(p) = &self.preview {
            let report = gif::analyze(&p.frames, &s.gif);
            if report.soft_alpha {
                warn(
                    ui,
                    "Some pixels are semi-transparent; GIF makes them fully opaque or transparent.",
                );
            }
            if report.reduced {
                warn(ui, "More than 255 colors: they will be reduced to 255.");
            } else {
                hint(
                    ui,
                    &format!("{} colors — kept exactly.", report.source_colors),
                );
            }
            if p.frames
                .iter()
                .any(|f| f.duration_ms % 10 != 0 || f.duration_ms < 20)
            {
                hint(
                    ui,
                    "GIF stores delays in 10 ms steps (min. 20 ms); durations are rounded.",
                );
            }
            if p.frames.len() == 1 {
                hint(ui, "Only one frame: the GIF will be a still image.");
            }
        }
    }

    fn sheet_options(&mut self, ui: &mut egui::Ui, s: &mut ExportSettings, count: usize) {
        section(ui, "Layout");
        let o = &mut s.sheet;
        egui::ComboBox::from_id_salt("sheet-layout")
            .selected_text(o.layout.label())
            .show_ui(ui, |ui| {
                for l in SheetLayout::ALL {
                    ui.selectable_value(&mut o.layout, l, l.label());
                }
            });
        ui.horizontal(|ui| match o.layout {
            SheetLayout::Columns => {
                ui.label("Columns");
                ui.add(egui::DragValue::new(&mut o.columns).range(1..=count.max(1) as u32));
            }
            SheetLayout::Rows => {
                ui.label("Rows");
                ui.add(egui::DragValue::new(&mut o.rows).range(1..=count.max(1) as u32));
            }
            _ => {}
        });
        egui::Grid::new("sheet-spacing")
            .num_columns(4)
            .show(ui, |ui| {
                ui.label("Spacing");
                ui.add(
                    egui::DragValue::new(&mut o.spacing)
                        .range(0..=MAX_SPACING)
                        .suffix(" px"),
                );
                ui.label("Padding");
                ui.add(
                    egui::DragValue::new(&mut o.padding)
                        .range(0..=MAX_SPACING)
                        .suffix(" px"),
                );
                ui.end_row();
                ui.label("Extrude");
                ui.add(
                    egui::DragValue::new(&mut o.extrude)
                        .range(0..=MAX_EXTRUDE)
                        .suffix(" px"),
                )
                .on_hover_text(
                    "Repeat edge pixels around each frame to stop texture bleeding in engines.",
                );
                ui.checkbox(&mut o.trim, "Trim empty borders")
                    .on_hover_text(
                        "Cut transparent borders per frame; offsets go into the metadata.",
                    );
                ui.end_row();
            });
        ui.horizontal(|ui| {
            ui.label("Background");
            let mut solid = o.background.is_some();
            ui.radio_value(&mut solid, false, "Transparent");
            ui.radio_value(&mut solid, true, "Color");
            if solid {
                let c = o.background.unwrap_or(Rgba::BLACK);
                let mut rgba = c.0;
                ui.color_edit_button_srgba_unmultiplied(&mut rgba);
                o.background = Some(Rgba(rgba));
            } else {
                o.background = None;
            }
        });
        ui.horizontal(|ui| {
            ui.label("Metadata");
            for m in [
                MetadataFormat::Kagero,
                MetadataFormat::JsonHash,
                MetadataFormat::None,
            ] {
                ui.radio_value(&mut o.metadata, m, m.label());
            }
        });
    }

    /// Output size of the current choices, checked against the limits.
    fn output_info(&self, s: &ExportSettings) -> Result<String, String> {
        let p = self
            .preview
            .as_ref()
            .ok_or_else(|| "Nothing to export".to_owned())?;
        let first = p
            .frames
            .first()
            .ok_or_else(|| "Nothing to export".to_owned())?;
        let (w, h) = first.image.dimensions();
        let scale = s.scale();
        match s.kind {
            ExportKind::Image | ExportKind::Animation => {
                let (sw, sh) = scaled_size(w, h, scale).map_err(|e| e.to_string())?;
                let what = if s.kind == ExportKind::Animation {
                    let total: u64 = p.frames.iter().map(|f| u64::from(f.duration_ms)).sum();
                    format!("{} frames · {:.2} s", p.frames.len(), total as f64 / 1000.0)
                } else {
                    format!("{} file(s)", p.frames.len())
                };
                Ok(format!("{w}×{h} → {sw}×{sh} px · {what}"))
            }
            ExportKind::SpriteSheet => {
                let plan = spritesheet::plan(&p.frames, &s.sheet).map_err(|e| e.to_string())?;
                let (sw, sh) = spritesheet::sheet_size(
                    plan.cell_w * scale,
                    plan.cell_h * scale,
                    plan.columns,
                    plan.rows,
                    &s.sheet,
                );
                check_output_size(sw, sh).map_err(|e| e.to_string())?;
                Ok(format!(
                    "{} frames · {}×{} grid · {sw}×{sh} px",
                    p.frames.len(),
                    plan.columns,
                    plan.rows
                ))
            }
        }
    }

    fn preview_ui(&mut self, ui: &mut egui::Ui, s: &ExportSettings) {
        section(ui, "Preview");
        let ctx = ui.ctx().clone();
        let Some(p) = &mut self.preview else {
            hint(ui, "No preview");
            return;
        };
        let (tex, size) = match s.kind {
            ExportKind::SpriteSheet => {
                let key = format!("{:?}", s.sheet);
                if p.sheet.as_ref().is_none_or(|(k, _)| *k != key) {
                    let opts = s.sheet;
                    let built = spritesheet::plan(&p.frames, &opts)
                        .and_then(|plan| spritesheet::compose(&p.frames, &plan, &opts));
                    p.sheet = built
                        .ok()
                        .map(|img| (key, texture(&ctx, "export-sheet-preview", &img)));
                }
                match &p.sheet {
                    Some((_, t)) => (t.clone(), t.size_vec2()),
                    None => {
                        hint(ui, "The sheet is too large to preview.");
                        return;
                    }
                }
            }
            ExportKind::Animation if p.textures.len() > 1 => {
                // Play the frames with their own durations (or GIF-rounded ones).
                let total: u64 = p
                    .frames
                    .iter()
                    .map(|f| u64::from(gif::delay_cs(f.duration_ms)) * 10)
                    .sum();
                let t = (p.started.elapsed().as_millis() as u64) % total.max(1);
                let mut acc = 0;
                let mut index = 0;
                for (i, f) in p.frames.iter().enumerate() {
                    acc += u64::from(gif::delay_cs(f.duration_ms)) * 10;
                    if t < acc {
                        index = i;
                        break;
                    }
                }
                ctx.request_repaint_after(std::time::Duration::from_millis(20));
                (p.textures[index].clone(), p.textures[index].size_vec2())
            }
            _ => match p.textures.first() {
                Some(t) => (t.clone(), t.size_vec2()),
                None => return,
            },
        };
        let fit = (PREVIEW.x / size.x).min(PREVIEW.y / size.y);
        let fit = if fit >= 1.0 { fit.floor() } else { fit };
        let shown = size * fit;
        let (rect, _) = ui.allocate_exact_size(PREVIEW, egui::Sense::hover());
        let painter = ui.painter_at(rect);
        painter.rect_filled(rect, 0.0, theme::BACKGROUND);
        let img_rect = egui::Rect::from_center_size(rect.center(), shown);
        // Checkerboard behind transparency.
        let cell = 8.0;
        let (cols, rows) = (
            (img_rect.width() / cell).ceil() as i32,
            (img_rect.height() / cell).ceil() as i32,
        );
        for cy in 0..rows {
            for cx in 0..cols {
                let c = if (cx + cy) % 2 == 0 {
                    theme::CHECKER_LIGHT
                } else {
                    theme::CHECKER_DARK
                };
                let r = egui::Rect::from_min_size(
                    img_rect.min + Vec2::new(cx as f32 * cell, cy as f32 * cell),
                    Vec2::splat(cell),
                )
                .intersect(img_rect);
                painter.rect_filled(r, 0.0, c);
            }
        }
        if s.kind == ExportKind::Image && s.still_format == StillFormat::Jpeg {
            let bg = s.jpeg.background;
            painter.rect_filled(img_rect, 0.0, Color32::from_rgb(bg.r(), bg.g(), bg.b()));
        }
        painter.image(
            tex.id(),
            img_rect,
            egui::Rect::from_min_max(egui::Pos2::ZERO, egui::Pos2::new(1.0, 1.0)),
            Color32::WHITE,
        );
        match self.output_info(s) {
            Ok(info) => {
                ui.label(RichText::new(info).small().color(theme::TEXT_PRIMARY));
            }
            Err(e) => {
                ui.label(RichText::new(e).small().color(theme::ERROR));
            }
        }
    }

    /// Builds the request for the current choices (without starting it).
    fn build_request(
        &self,
        editor: &Editor,
        s: &ExportSettings,
        dir: PathBuf,
    ) -> Result<ExportRequest, String> {
        let p = self
            .preview
            .as_ref()
            .ok_or_else(|| "Nothing to export".to_owned())?;
        self.output_info(s)?;
        let base = sequence::base_name(&s.naming, editor.name());
        let target = match s.kind {
            ExportKind::Image => {
                let names = sequence::file_names(
                    &s.naming,
                    &base,
                    p.frames.len(),
                    s.still_format.extension(),
                );
                ExportTarget::Images {
                    format: s.still_format,
                    paths: sequence::paths_in(&dir, &names),
                }
            }
            ExportKind::Animation => ExportTarget::Gif {
                path: dir.join(format!("{base}.gif")),
            },
            ExportKind::SpriteSheet => ExportTarget::Sheet {
                image_path: dir.join(format!("{base}.{}", s.sheet.image_format.extension())),
                json_path: (s.sheet.metadata != MetadataFormat::None)
                    .then(|| dir.join(format!("{base}.json"))),
            },
        };
        Ok(ExportRequest {
            frames: p.frames.clone(),
            scale: s.scale(),
            jpeg: s.jpeg,
            gif: s.gif,
            sheet: s.sheet,
            base_name: base,
            digits: s.naming.digits,
            target,
        })
    }

    fn start(&mut self, ctx: &egui::Context, request: ExportRequest) {
        let repaint_ctx = ctx.clone();
        self.result = None;
        self.message = None;
        self.job = Some(ExportJob::spawn(
            request,
            Arc::new(move || repaint_ctx.request_repaint()),
        ));
    }

    fn footer(
        &mut self,
        ui: &mut egui::Ui,
        editor: &Editor,
        s: &mut ExportSettings,
        frames: &[usize],
    ) {
        let ctx = ui.ctx().clone();
        // Destination folder.
        let default_dir = s.last_dir.clone().filter(|d| d.is_dir()).or_else(|| {
            editor
                .path()
                .and_then(|p| p.parent())
                .map(|p| p.to_path_buf())
        });
        ui.horizontal(|ui| {
            ui.label("Folder");
            let text = default_dir.as_ref().map_or_else(
                || "Choose a folder…".to_owned(),
                |d| d.display().to_string(),
            );
            ui.label(
                RichText::new(text)
                    .monospace()
                    .small()
                    .color(theme::TEXT_SECONDARY),
            );
            if ui
                .add_enabled(!self.is_running(), egui::Button::new("Browse…"))
                .clicked()
            {
                if let Some(dir) = native::pick_folder(default_dir.as_deref(), "Export to folder") {
                    s.last_dir = Some(dir);
                }
            }
        });
        ui.checkbox(&mut s.open_folder_after, "Open the folder after exporting");

        if let Some(job) = &self.job {
            let st = job.state();
            let fraction = st.done as f32 / st.total.max(1) as f32;
            ui.add(
                egui::ProgressBar::new(fraction)
                    .text(format!("Exporting… {}/{}", st.done, st.total)),
            );
        }
        if let Some((request, existing)) = &self.confirm {
            warn(
                ui,
                &format!("{existing} file(s) already exist and will be replaced."),
            );
            let request = request.clone();
            ui.horizontal(|ui| {
                if ui.add(accent_button("Replace")).clicked() {
                    self.confirm = None;
                    self.start(&ctx, request);
                }
                if ui.button("Cancel").clicked() {
                    self.confirm = None;
                }
            });
        }
        match &self.result {
            Some(Ok(summary)) => {
                let folder = summary
                    .files
                    .first()
                    .and_then(|f| f.parent())
                    .map(|p| p.to_path_buf());
                ui.label(
                    RichText::new(format!("Exported {} file(s).", summary.files.len()))
                        .color(theme::SUCCESS),
                );
                for note in &summary.notes {
                    hint(ui, note);
                }
                if let Some(folder) = folder {
                    if ui.button("Open Folder").clicked() {
                        open_folder(&folder);
                    }
                }
            }
            Some(Err(e)) => {
                ui.label(RichText::new(e).color(theme::ERROR));
            }
            None => {}
        }
        if let Some(m) = &self.message {
            ui.label(RichText::new(m).color(theme::ERROR));
        }

        ui.add_space(4.0);
        ui.horizontal(|ui| {
            if self.is_running() {
                if ui.button("Cancel Export").clicked() {
                    if let Some(job) = &self.job {
                        job.cancel();
                    }
                }
                return;
            }
            let ready = default_dir.is_some() && !frames.is_empty() && self.confirm.is_none();
            if ui.add_enabled(ready, accent_button("Export")).clicked() {
                if let Some(dir) = default_dir.clone() {
                    match self.build_request(editor, s, dir.clone()) {
                        Ok(request) => {
                            s.last_dir = Some(dir);
                            let existing = sequence::existing(&request.output_paths()).len();
                            if existing > 0 {
                                self.confirm = Some((request, existing));
                            } else {
                                self.start(&ctx, request);
                            }
                        }
                        Err(e) => self.message = Some(e),
                    }
                }
            }
            if ui.button("Close").clicked() {
                self.open = false;
            }
        });
    }
}

fn section(ui: &mut egui::Ui, title: &str) {
    ui.add_space(6.0);
    ui.label(
        RichText::new(title.to_uppercase())
            .small()
            .strong()
            .color(theme::TEXT_SECONDARY),
    );
}

fn hint(ui: &mut egui::Ui, text: &str) {
    ui.label(RichText::new(text).small().color(theme::TEXT_SECONDARY));
}

fn warn(ui: &mut egui::Ui, text: &str) {
    ui.label(RichText::new(text).small().color(theme::ACCENT));
}

/// Opens a folder in the system file manager.
pub fn open_folder(folder: &std::path::Path) {
    let program = if cfg!(windows) {
        "explorer"
    } else if cfg!(target_os = "macos") {
        "open"
    } else {
        "xdg-open"
    };
    if let Err(e) = std::process::Command::new(program).arg(folder).spawn() {
        tracing::warn!("could not open {}: {e}", folder.display());
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::{Document, Rgba};
    use crate::editor::ColorSlot;
    use crate::export::{SheetLayout, StillFormat};

    /// A 4×3 document with three frames, one pixel drawn on the first.
    fn editor() -> Editor {
        let doc = Document::new(4, 3, Rgba::TRANSPARENT).expect("valid");
        let mut e = Editor::new(doc, None, "Ember Spirit".into());
        e.begin_stroke((1, 1), ColorSlot::Primary).expect("draw");
        e.end_stroke();
        e.duplicate_frame().expect("frame");
        e.add_frame().expect("frame");
        e
    }

    /// Runs the dialog for a couple of frames, the way the app does.
    fn show(dialog: &mut ExportDialog, e: &Editor, s: &mut ExportSettings) {
        let ctx = egui::Context::default();
        for _ in 0..2 {
            let _ = ctx.run(egui::RawInput::default(), |ctx| {
                dialog.show(ctx, e, &[0, 2], s);
            });
        }
    }

    #[test]
    fn every_tab_renders_and_builds_a_request() {
        let e = editor();
        let dir = std::env::temp_dir().join("kagero-dialog-test");
        let mut s = ExportSettings {
            frames: FrameSelection::All,
            ..ExportSettings::default()
        };
        for kind in [
            ExportKind::Image,
            ExportKind::Animation,
            ExportKind::SpriteSheet,
        ] {
            let mut dialog = ExportDialog::default();
            dialog.open_at(kind, &mut s);
            show(&mut dialog, &e, &mut s);
            assert!(dialog.open);
            let p = dialog.preview.as_ref().expect("preview built");
            assert_eq!(p.frames.len(), 3);
            let req = dialog
                .build_request(&e, &s, dir.clone())
                .expect("valid request");
            let paths = req.output_paths();
            match kind {
                ExportKind::Image => {
                    assert_eq!(paths.len(), 3);
                    assert!(paths[0].ends_with("Ember Spirit_0001.png"), "{paths:?}");
                }
                ExportKind::Animation => {
                    assert!(paths[0].ends_with("Ember Spirit.gif"), "{paths:?}");
                }
                ExportKind::SpriteSheet => {
                    assert_eq!(paths.len(), 2, "image + JSON: {paths:?}");
                    assert!(p.sheet.is_some(), "sheet preview");
                }
            }
        }
    }

    #[test]
    fn selected_frames_and_formats() {
        let e = editor();
        let mut s = ExportSettings {
            frames: FrameSelection::Selected,
            still_format: StillFormat::Jpeg,
            ..ExportSettings::default()
        };
        let mut dialog = ExportDialog::default();
        dialog.open_at(ExportKind::Image, &mut s);
        show(&mut dialog, &e, &mut s);
        let p = dialog.preview.as_ref().expect("preview");
        let indices: Vec<usize> = p.frames.iter().map(|f| f.index).collect();
        assert_eq!(indices, vec![0, 2]);
        let req = dialog
            .build_request(&e, &s, std::env::temp_dir())
            .expect("request");
        assert!(req.output_paths()[0]
            .extension()
            .is_some_and(|x| x == "jpg"));
    }

    #[test]
    fn oversized_output_is_refused_before_export() {
        let e = editor();
        let mut s = ExportSettings {
            frames: FrameSelection::All,
            ..ExportSettings::default()
        };
        s.sheet.layout = SheetLayout::Horizontal;
        s.sheet.spacing = crate::export::options::MAX_SPACING;
        s.scale_sheet = crate::export::MAX_SCALE;
        let mut dialog = ExportDialog::default();
        dialog.open_at(ExportKind::SpriteSheet, &mut s);
        show(&mut dialog, &e, &mut s);
        // Padding beyond the 16384 px limit must be caught by the size check.
        s.sheet.padding = 100_000;
        let result = dialog.output_info(&s);
        if let Ok(info) = &result {
            panic!("expected a size error, got {info}");
        }
    }
}
