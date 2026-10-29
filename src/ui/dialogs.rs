//! Modal dialogs: new document, unsaved changes, errors, about.

use egui::emath::GuiRounding as _;
use egui::{Button, Id, Modal, RichText, Vec2};

use crate::core::{Rgba, MAX_CANVAS_SIZE};
use crate::theme;

/// Background of a new document.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Background {
    Transparent,
    White,
    Black,
}

impl Background {
    pub fn color(self) -> Rgba {
        match self {
            Background::Transparent => Rgba::TRANSPARENT,
            Background::White => Rgba::WHITE,
            Background::Black => Rgba::BLACK,
        }
    }

    fn name(self) -> &'static str {
        match self {
            Background::Transparent => "Transparent",
            Background::White => "White",
            Background::Black => "Black",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NewDocumentState {
    pub width: u32,
    pub height: u32,
    pub background: Background,
}

pub enum NewDocumentResult {
    Open,
    Create,
    Cancel,
}

/// Named starting sizes.
const PRESETS: [(&str, u32, u32); 8] = [
    ("Icon", 16, 16),
    ("Sprite", 32, 32),
    ("Character", 64, 64),
    ("Portrait", 128, 128),
    ("Large", 256, 256),
    ("Tile strip", 128, 32),
    ("Screen", 320, 180),
    ("Wide screen", 640, 360),
];

pub fn new_document(ctx: &egui::Context, state: &mut NewDocumentState) -> NewDocumentResult {
    let mut result = NewDocumentResult::Open;
    let modal = Modal::new(Id::new("new-document"))
        .frame(
            egui::Frame::new()
                .fill(theme::PANEL)
                .stroke(egui::Stroke::new(1.0_f32, theme::BEVEL_LIGHT))
                .shadow(egui::Shadow {
                    offset: [6, 6],
                    blur: 0,
                    spread: 0,
                    color: egui::Color32::from_black_alpha(170),
                })
                .inner_margin(egui::Margin::same(18)),
        )
        .show(ctx, |ui| {
            ui.set_width(600.0);
            ui.label(
                RichText::new("New Image")
                    .font(theme::font(2))
                    .color(theme::TEXT_PRIMARY),
            );
            ui.label(
                RichText::new("Pick a preset or type any size, then choose the background.")
                    .color(theme::TEXT_SECONDARY),
            );
            ui.add_space(10.0);
            ui.horizontal_top(|ui| {
                ui.vertical(|ui| {
                    ui.set_width(300.0);
                    caption(ui, "Presets");
                    egui::Grid::new("new-doc-presets")
                        .num_columns(2)
                        .spacing([8.0, 8.0])
                        .show(ui, |ui| {
                            for (i, (name, w, h)) in PRESETS.into_iter().enumerate() {
                                let selected = state.width == w && state.height == h;
                                if preset_card(ui, name, w, h, selected).clicked() {
                                    state.width = w;
                                    state.height = h;
                                }
                                if i % 2 == 1 {
                                    ui.end_row();
                                }
                            }
                        });
                });
                ui.add_space(14.0);
                ui.vertical(|ui| {
                    caption(ui, "Size");
                    ui.horizontal(|ui| {
                        ui.add(
                            egui::DragValue::new(&mut state.width)
                                .range(1..=MAX_CANVAS_SIZE)
                                .suffix(" px"),
                        )
                        .on_hover_text("Width");
                        if super::icons::icon_button(
                            ui,
                            super::icons::Icon::Swap,
                            24.0,
                            false,
                            state.width != state.height,
                            "Swap width and height",
                        )
                        .clicked()
                        {
                            std::mem::swap(&mut state.width, &mut state.height);
                        }
                        ui.add(
                            egui::DragValue::new(&mut state.height)
                                .range(1..=MAX_CANVAS_SIZE)
                                .suffix(" px"),
                        )
                        .on_hover_text("Height");
                    });
                    ui.label(
                        RichText::new(format!("1 to {MAX_CANVAS_SIZE} px per side"))
                            .small()
                            .color(theme::TEXT_SECONDARY),
                    );
                    ui.add_space(8.0);
                    caption(ui, "Background");
                    ui.horizontal(|ui| {
                        for bg in [
                            Background::Transparent,
                            Background::White,
                            Background::Black,
                        ] {
                            if background_swatch(ui, bg, state.background == bg).clicked() {
                                state.background = bg;
                            }
                        }
                    });
                    ui.add_space(8.0);
                    caption(ui, "Preview");
                    preview(ui, state);
                });
            });
            ui.add_space(12.0);
            ui.separator();
            ui.horizontal(|ui| {
                if ui.add(accent_button("Create")).clicked() {
                    result = NewDocumentResult::Create;
                }
                if ui.button("Cancel").clicked() {
                    result = NewDocumentResult::Cancel;
                }
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    ui.label(
                        RichText::new("Enter creates · Esc cancels")
                            .small()
                            .color(theme::TEXT_SECONDARY),
                    );
                });
            });
            if ui.input(|i| i.key_pressed(egui::Key::Enter)) && !ui.ctx().wants_keyboard_input() {
                result = NewDocumentResult::Create;
            }
        });
    if modal.should_close() && matches!(result, NewDocumentResult::Open) {
        result = NewDocumentResult::Cancel;
    }
    state.width = state.width.clamp(1, MAX_CANVAS_SIZE);
    state.height = state.height.clamp(1, MAX_CANVAS_SIZE);
    result
}

fn caption(ui: &mut egui::Ui, text: &str) {
    ui.label(
        RichText::new(text.to_uppercase())
            .small()
            .strong()
            .color(theme::ACCENT),
    );
}

/// A preset button: a tiny outline of its shape, the name and the size.
fn preset_card(ui: &mut egui::Ui, name: &str, w: u32, h: u32, selected: bool) -> egui::Response {
    let (rect, resp) = ui.allocate_exact_size(Vec2::new(146.0, 48.0), egui::Sense::click());
    let painter = ui.painter();
    let fill = if selected {
        theme::EMBER_DEEP
    } else if resp.hovered() {
        theme::SURFACE_HOVER
    } else {
        theme::SURFACE
    };
    painter.rect_filled(rect, 0.0, fill);
    let outline = if selected {
        theme::ACCENT
    } else {
        theme::BEVEL_DARK
    };
    painter.rect_stroke(
        rect,
        0.0,
        egui::Stroke::new(1.0_f32, outline),
        egui::StrokeKind::Inside,
    );
    // Shape thumbnail, fitted in a 28 px box.
    let scale = 28.0 / w.max(h) as f32;
    let shape = egui::Rect::from_center_size(
        egui::pos2(rect.left() + 24.0, rect.center().y),
        Vec2::new((w as f32 * scale).max(3.0), (h as f32 * scale).max(3.0)),
    );
    painter.rect_filled(shape, 0.0, theme::CHECKER_LIGHT);
    painter.rect_stroke(
        shape,
        0.0,
        egui::Stroke::new(
            1.0_f32,
            if selected {
                theme::ACCENT
            } else {
                theme::TEXT_SECONDARY
            },
        ),
        egui::StrokeKind::Inside,
    );
    let text_x = rect.left() + 46.0;
    painter.text(
        egui::pos2(text_x, rect.top() + 8.0),
        egui::Align2::LEFT_TOP,
        name,
        egui::TextStyle::Body.resolve(ui.style()),
        theme::TEXT_PRIMARY,
    );
    painter.text(
        egui::pos2(text_x, rect.bottom() - 8.0),
        egui::Align2::LEFT_BOTTOM,
        format!("{w}×{h}"),
        egui::TextStyle::Small.resolve(ui.style()),
        if selected {
            theme::ACCENT
        } else {
            theme::TEXT_SECONDARY
        },
    );
    resp.on_hover_cursor(egui::CursorIcon::PointingHand)
}

/// Paints `rect` as a background: a checkerboard for transparent, else the color.
fn paint_background(painter: &egui::Painter, rect: egui::Rect, bg: Background, cell: f32) {
    match bg {
        Background::Transparent => {
            painter.rect_filled(rect, 0.0, theme::CHECKER_LIGHT);
            let mut mesh = egui::Mesh::default();
            let (cols, rows) = (
                (rect.width() / cell).ceil() as i32,
                (rect.height() / cell).ceil() as i32,
            );
            for y in 0..rows {
                for x in 0..cols {
                    if (x + y) % 2 == 0 {
                        continue;
                    }
                    let min = rect.min + Vec2::new(x as f32, y as f32) * cell;
                    let r = egui::Rect::from_min_size(min, Vec2::splat(cell)).intersect(rect);
                    mesh.add_colored_rect(r, theme::CHECKER_DARK);
                }
            }
            painter.add(egui::Shape::mesh(mesh));
        }
        Background::White => {
            painter.rect_filled(rect, 0.0, egui::Color32::WHITE);
        }
        Background::Black => {
            painter.rect_filled(rect, 0.0, egui::Color32::BLACK);
        }
    }
}

fn background_swatch(ui: &mut egui::Ui, bg: Background, selected: bool) -> egui::Response {
    let resp = ui
        .vertical(|ui| {
            let (rect, resp) = ui.allocate_exact_size(Vec2::splat(40.0), egui::Sense::click());
            paint_background(ui.painter(), rect.shrink(3.0), bg, 6.0);
            let stroke = if selected {
                egui::Stroke::new(2.0_f32, theme::ACCENT)
            } else if resp.hovered() {
                egui::Stroke::new(1.0_f32, theme::TEXT_SECONDARY)
            } else {
                egui::Stroke::new(1.0_f32, theme::BEVEL_DARK)
            };
            ui.painter()
                .rect_stroke(rect, 0.0, stroke, egui::StrokeKind::Inside);
            ui.label(RichText::new(bg.name()).small().color(if selected {
                theme::ACCENT
            } else {
                theme::TEXT_SECONDARY
            }));
            resp
        })
        .inner;
    resp.on_hover_cursor(egui::CursorIcon::PointingHand)
}

/// The new image's shape and background, fitted in a box, with its size.
fn preview(ui: &mut egui::Ui, state: &NewDocumentState) {
    let (area, _) = ui.allocate_exact_size(Vec2::new(250.0, 110.0), egui::Sense::hover());
    let painter = ui.painter();
    painter.rect_filled(area, 0.0, theme::BACKGROUND);
    let (w, h) = (state.width.max(1) as f32, state.height.max(1) as f32);
    let scale = ((area.width() - 20.0) / w).min((area.height() - 20.0) / h);
    let size = Vec2::new((w * scale).max(2.0), (h * scale).max(2.0));
    let rect = egui::Rect::from_center_size(area.center(), size)
        .round_to_pixels(ui.ctx().pixels_per_point());
    paint_background(painter, rect, state.background, scale.clamp(4.0, 10.0));
    painter.rect_stroke(
        rect,
        0.0,
        egui::Stroke::new(1.0_f32, theme::BORDER),
        egui::StrokeKind::Outside,
    );
    let bytes = u64::from(state.width) * u64::from(state.height) * 4;
    let memory = if bytes >= 1024 * 1024 {
        format!("{:.1} MiB", bytes as f64 / (1024.0 * 1024.0))
    } else {
        format!("{} KiB", bytes.div_ceil(1024))
    };
    ui.label(
        RichText::new(format!(
            "{} × {} px · {memory} per frame and layer",
            state.width, state.height
        ))
        .small()
        .color(theme::TEXT_SECONDARY),
    );
}

pub enum UnsavedChoice {
    Pending,
    Save,
    Discard,
    Cancel,
}

pub fn unsaved_changes(ctx: &egui::Context, name: &str) -> UnsavedChoice {
    let mut choice = UnsavedChoice::Pending;
    let modal = Modal::new(Id::new("unsaved-changes")).show(ctx, |ui| {
        ui.set_width(380.0);
        ui.heading("Unsaved changes");
        ui.add_space(6.0);
        ui.label(format!("Save changes to “{name}” before continuing?"));
        ui.label(
            RichText::new("Changes that are not saved will be lost.")
                .small()
                .color(theme::TEXT_SECONDARY),
        );
        ui.add_space(12.0);
        ui.horizontal(|ui| {
            if ui.add(accent_button("Save")).clicked() {
                choice = UnsavedChoice::Save;
            }
            if ui.button("Don't Save").clicked() {
                choice = UnsavedChoice::Discard;
            }
            if ui.button("Cancel").clicked() {
                choice = UnsavedChoice::Cancel;
            }
        });
    });
    if modal.should_close() && matches!(choice, UnsavedChoice::Pending) {
        choice = UnsavedChoice::Cancel;
    }
    choice
}

pub enum ConfirmChoice {
    Pending,
    Confirm,
    Cancel,
}

/// A yes/no question with a destructive or important confirm action.
pub fn confirm(
    ctx: &egui::Context,
    title: &str,
    message: &str,
    confirm_label: &str,
) -> ConfirmChoice {
    let mut choice = ConfirmChoice::Pending;
    let modal = Modal::new(Id::new(("confirm-dialog", title))).show(ctx, |ui| {
        ui.set_width(400.0);
        ui.heading(title);
        ui.add_space(6.0);
        ui.label(message);
        ui.add_space(12.0);
        ui.horizontal(|ui| {
            if ui.add(accent_button(confirm_label)).clicked() {
                choice = ConfirmChoice::Confirm;
            }
            if ui.button("Cancel").clicked() {
                choice = ConfirmChoice::Cancel;
            }
        });
    });
    if modal.should_close() && matches!(choice, ConfirmChoice::Pending) {
        choice = ConfirmChoice::Cancel;
    }
    choice
}

/// Shows an error; returns `true` when dismissed.
pub fn error(ctx: &egui::Context, title: &str, message: &str) -> bool {
    let mut close = false;
    let modal = Modal::new(Id::new("error-dialog")).show(ctx, |ui| {
        ui.set_width(420.0);
        ui.label(RichText::new(title).heading().color(theme::ERROR));
        ui.add_space(6.0);
        ui.label(message);
        ui.add_space(12.0);
        if ui.add(accent_button("OK")).clicked() {
            close = true;
        }
    });
    close || modal.should_close()
}

/// Shows the About box; returns `true` when dismissed.
/// What the About window asks for.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AboutChoice {
    Keep,
    Close,
    Licenses,
}

pub fn about(ctx: &egui::Context) -> AboutChoice {
    let mut close = false;
    let mut licenses = false;
    let modal = Modal::new(Id::new("about-dialog"))
        .frame(
            egui::Frame::new()
                .fill(theme::PANEL)
                .stroke(egui::Stroke::new(1.0_f32, theme::BEVEL_LIGHT))
                .shadow(egui::Shadow {
                    offset: [6, 6],
                    blur: 0,
                    spread: 0,
                    color: egui::Color32::from_black_alpha(170),
                })
                .inner_margin(egui::Margin::ZERO),
        )
        .show(ctx, |ui| {
            ui.set_width(560.0);
            ui.spacing_mut().item_spacing.y = 4.0;
            about_banner(ui);
            egui::Frame::new()
                .inner_margin(egui::Margin::symmetric(20, 14))
                .show(ui, |ui| {
                    ui.label(format!(
                        "An independent pixel-art and sprite-animation editor by {}.",
                        crate::STUDIO_NAME
                    ));
                    ui.add_space(6.0);
                    let repo = format!(
                        "https://github.com/{}/{}",
                        crate::release::GITHUB_OWNER,
                        crate::release::GITHUB_REPO
                    );
                    egui::Grid::new("about-facts")
                        .num_columns(2)
                        .spacing([16.0, 4.0])
                        .show(ui, |ui| {
                            fact(ui, "Studio", |ui| {
                                ui.label(crate::STUDIO_NAME);
                            });
                            fact(ui, "License", |ui| {
                                ui.label("MIT (dependencies: THIRD_PARTY_LICENSES.md)");
                            });
                            fact(ui, "Made with", |ui| {
                                ui.label("Rust, egui and the Kagero Pixel font");
                            });
                            fact(ui, "Links", |ui| {
                                ui.hyperlink_to(
                                    "Documentation",
                                    "https://kagero.gitbook.io/kagero-docs",
                                );
                                ui.label(RichText::new("·").color(theme::TEXT_SECONDARY));
                                ui.hyperlink_to(
                                    "Changelog",
                                    format!("{repo}/blob/main/CHANGELOG.md"),
                                );
                                ui.label(RichText::new("·").color(theme::TEXT_SECONDARY));
                                ui.hyperlink_to("GitHub", repo.clone());
                            });
                        });

                    ui.add_space(12.0);
                    ui.label(RichText::new("SHORTCUTS").color(theme::TEXT_SECONDARY));
                    ui.add_space(2.0);
                    egui::Grid::new("about-shortcuts")
                        .num_columns(4)
                        .spacing([10.0, 5.0])
                        .show(ui, |ui| {
                            let rows = [
                                ("B E G I", "Draw, erase, fill, pick", "M", "Select"),
                                ("L U", "Line, rectangle", "Ctrl+C Ctrl+V", "Copy, paste"),
                                ("Space", "Pan (hold)", "Ctrl+Z Ctrl+Y", "Undo, redo"),
                                ("0 1", "Fit, 100%", "Ctrl+Shift+E", "Export"),
                                ("X", "Swap colors", "Enter", "Play / stop"),
                                ("O", "Onion skin", "Alt+N Alt+D", "New, duplicate frame"),
                            ];
                            for (k1, v1, k2, v2) in rows {
                                keycaps(ui, k1);
                                ui.label(RichText::new(v1).color(theme::TEXT_SECONDARY));
                                keycaps(ui, k2);
                                ui.label(RichText::new(v2).color(theme::TEXT_SECONDARY));
                                ui.end_row();
                            }
                        });

                    ui.add_space(14.0);
                    ui.horizontal(|ui| {
                        ui.label(
                            RichText::new(format!("© 2026 {}", crate::STUDIO_NAME))
                                .color(theme::TEXT_SECONDARY),
                        );
                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            if ui.add(accent_button("Close")).clicked() {
                                close = true;
                            }
                            if ui.button("Licenses").clicked() {
                                licenses = true;
                            }
                        });
                    });
                });
        });
    if licenses {
        AboutChoice::Licenses
    } else if close || modal.should_close() {
        AboutChoice::Close
    } else {
        AboutChoice::Keep
    }
}

/// Kagero's license and the third-party list, embedded so they ship with the program.
const LICENSE_TEXT: &str = include_str!("../../LICENSE");
const THIRD_PARTY_TEXT: &str = include_str!("../../THIRD_PARTY_LICENSES.md");

/// The licenses window. `tab`: 0 = Kagero, 1 = third-party components. Returns
/// `false` when closed.
pub fn licenses(ctx: &egui::Context, tab: &mut u8) -> bool {
    let mut close = false;
    let height = (ctx.screen_rect().height() - 220.0).clamp(240.0, 520.0);
    let modal = Modal::new(Id::new("licenses-dialog")).show(ctx, |ui| {
        ui.set_width(680.0);
        ui.label(
            RichText::new("Licenses")
                .font(theme::font(2))
                .color(theme::TEXT_PRIMARY),
        );
        ui.label(
            RichText::new(
                "Kagero is MIT-licensed. It is built on open-source components whose licenses are listed here.",
            )
            .color(theme::TEXT_SECONDARY),
        );
        ui.add_space(6.0);
        ui.horizontal(|ui| {
            ui.selectable_value(tab, 0, "Kagero (MIT)");
            ui.selectable_value(tab, 1, "Third-party components");
        });
        ui.separator();
        let text = if *tab == 0 { LICENSE_TEXT } else { THIRD_PARTY_TEXT };
        egui::ScrollArea::vertical()
            .max_height(height)
            .auto_shrink([false, true])
            .show(ui, |ui| {
                ui.label(RichText::new(text).monospace().color(theme::TEXT_PRIMARY));
            });
        ui.add_space(8.0);
        if ui.add(accent_button("Close")).clicked() {
            close = true;
        }
    });
    !(close || modal.should_close())
}

/// The dark banner at the top of About: flame, name and version badge.
fn about_banner(ui: &mut egui::Ui) {
    egui::Frame::new()
        .fill(theme::BACKGROUND)
        .inner_margin(egui::Margin::symmetric(20, 16))
        .show(ui, |ui| {
            ui.set_width(ui.available_width());
            ui.horizontal(|ui| {
                let (rect, _) = ui.allocate_exact_size(Vec2::splat(64.0), egui::Sense::hover());
                super::icons::paint_bitmap(
                    ui.painter(),
                    rect,
                    &super::icons::LOGO,
                    super::icons::Tint::Normal,
                );
                ui.add_space(12.0);
                ui.vertical(|ui| {
                    ui.spacing_mut().item_spacing.y = 2.0;
                    ui.label(
                        RichText::new("Kagero")
                            .font(theme::font(3))
                            .color(theme::TEXT_PRIMARY),
                    );
                    ui.label(RichText::new("陽炎 · pixel art editor").color(theme::ACCENT));
                    ui.add_space(4.0);
                    egui::Frame::new()
                        .fill(theme::EMBER_DEEP)
                        .stroke(egui::Stroke::new(1.0_f32, theme::ACCENT))
                        .inner_margin(egui::Margin::symmetric(6, 2))
                        .show(ui, |ui| {
                            ui.label(
                                RichText::new(format!(
                                    "VERSION {} {}",
                                    crate::APP_VERSION,
                                    crate::APP_CHANNEL.to_uppercase()
                                ))
                                .color(theme::TEXT_PRIMARY),
                            );
                        });
                });
            });
        });
    // A 2-pixel ember rule between the banner and the body.
    let (line, _) =
        ui.allocate_exact_size(Vec2::new(ui.available_width(), 2.0), egui::Sense::hover());
    ui.painter().rect_filled(line, 0.0, theme::ACCENT_PRESSED);
}

fn fact(ui: &mut egui::Ui, name: &str, value: impl FnOnce(&mut egui::Ui)) {
    ui.label(RichText::new(name).color(theme::TEXT_SECONDARY));
    ui.horizontal(|ui| {
        ui.spacing_mut().item_spacing.x = 6.0;
        value(ui);
    });
    ui.end_row();
}

/// Space-separated keys drawn as small bevelled keycaps.
fn keycaps(ui: &mut egui::Ui, keys: &str) {
    ui.horizontal(|ui| {
        ui.spacing_mut().item_spacing.x = 3.0;
        for key in keys.split(' ') {
            let resp = egui::Frame::new()
                .fill(theme::SURFACE)
                .stroke(egui::Stroke::new(1.0_f32, theme::BEVEL_DARK))
                .inner_margin(egui::Margin::symmetric(5, 1))
                .show(ui, |ui| {
                    ui.label(RichText::new(key).color(theme::ACCENT));
                });
            super::icons::bevel(ui.painter(), resp.response.rect.shrink(1.0), true);
        }
    });
}

/// The primary call-to-action button style.
pub fn accent_button(text: &str) -> Button<'static> {
    Button::new(
        RichText::new(text.to_owned())
            .strong()
            .color(theme::ON_ACCENT),
    )
    .fill(theme::ACCENT)
    .min_size(Vec2::new(88.0, 28.0))
}
