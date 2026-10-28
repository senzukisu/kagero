//! Kagero's own window frame: a pixel-style title bar (flame, menus, document title,
//! minimize / maximize / close) and resize edges, used instead of the system frame.

use egui::{Color32, CursorIcon, Id, Rect, ResizeDirection, Sense, Vec2, ViewportCommand};

use super::icons::{paint_bitmap, Tint, LOGO};
use crate::theme;

/// Height of the title bar in points.
pub const HEIGHT: f32 = 32.0;
/// Width of each window button.
const BUTTON_WIDTH: f32 = 46.0;
/// How far from the window border a drag resizes the window.
const RESIZE_MARGIN: f32 = 5.0;

const MINIMIZE: [&str; 10] = [
    "..........",
    "..........",
    "..........",
    "..........",
    "..........",
    "..........",
    "..........",
    "##########",
    "..........",
    "..........",
];
const MAXIMIZE: [&str; 10] = [
    "##########",
    "##########",
    "#........#",
    "#........#",
    "#........#",
    "#........#",
    "#........#",
    "#........#",
    "#........#",
    "##########",
];
const RESTORE: [&str; 10] = [
    "..########",
    "..########",
    "..#......#",
    "########.#",
    "########.#",
    "#......#.#",
    "#......###",
    "#......#..",
    "#......#..",
    "########..",
];
const CLOSE: [&str; 10] = [
    "#........#",
    ".#......#.",
    "..#....#..",
    "...#..#...",
    "....##....",
    "....##....",
    "...#..#...",
    "..#....#..",
    ".#......#.",
    "#........#",
];

/// Draws the title bar. `menus` adds the menu bar after the flame.
pub fn show(ui: &mut egui::Ui, title: &str, menus: impl FnOnce(&mut egui::Ui)) {
    let ctx = ui.ctx().clone();
    let maximized = ctx.input(|i| i.viewport().maximized.unwrap_or(false));
    let bar = ui.max_rect();

    // The empty parts of the bar move the window; widgets drawn later take priority.
    let drag = ui.interact(bar, Id::new("title-bar-drag"), Sense::click_and_drag());
    if drag.double_clicked() {
        ctx.send_viewport_cmd(ViewportCommand::Maximized(!maximized));
    } else if drag.drag_started_by(egui::PointerButton::Primary) {
        ctx.send_viewport_cmd(ViewportCommand::StartDrag);
    }

    // Document title, centered on the whole bar.
    ui.painter().text(
        bar.center(),
        egui::Align2::CENTER_CENTER,
        title,
        egui::TextStyle::Body.resolve(ui.style()),
        theme::TEXT_SECONDARY,
    );

    ui.horizontal_centered(|ui| {
        ui.spacing_mut().item_spacing.x = 0.0;
        ui.add_space(10.0);
        let (logo, _) = ui.allocate_exact_size(Vec2::splat(16.0), Sense::hover());
        paint_bitmap(ui.painter(), logo, &LOGO, Tint::Normal);
        ui.add_space(8.0);
        menus(ui);

        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            if window_button(ui, &CLOSE, "Close", true).clicked() {
                ctx.send_viewport_cmd(ViewportCommand::Close);
            }
            let (icon, tip) = if maximized {
                (&RESTORE, "Restore down")
            } else {
                (&MAXIMIZE, "Maximize")
            };
            if window_button(ui, icon, tip, false).clicked() {
                ctx.send_viewport_cmd(ViewportCommand::Maximized(!maximized));
            }
            if window_button(ui, &MINIMIZE, "Minimize", false).clicked() {
                ctx.send_viewport_cmd(ViewportCommand::Minimized(true));
            }
        });
    });

    // A 1-pixel ember line under the bar.
    let px = 1.0 / ctx.pixels_per_point();
    let line = Rect::from_min_max(egui::pos2(bar.min.x, bar.max.y - px), bar.max);
    ui.painter().rect_filled(line, 0.0, theme::BORDER);
}

fn window_button(ui: &mut egui::Ui, icon: &[&str], tooltip: &str, close: bool) -> egui::Response {
    let height = ui.available_height().min(HEIGHT);
    let (rect, resp) = ui.allocate_exact_size(Vec2::new(BUTTON_WIDTH, height), Sense::click());
    let bg = if resp.is_pointer_button_down_on() {
        if close {
            Color32::from_rgb(0xB8, 0x3A, 0x3A)
        } else {
            theme::EMBER_DEEP
        }
    } else if resp.hovered() {
        if close {
            theme::ERROR
        } else {
            theme::SURFACE_HOVER
        }
    } else {
        Color32::TRANSPARENT
    };
    ui.painter().rect_filled(rect, 0.0, bg);
    let glyph = Rect::from_center_size(rect.center(), Vec2::splat(10.0));
    paint_bitmap(ui.painter(), glyph, icon, Tint::Normal);
    resp.on_hover_text(tooltip)
}

/// Lets the user resize the undecorated window from its borders. Call once per
/// frame before the panels; does nothing while maximized.
pub fn resize_edges(ctx: &egui::Context) {
    let maximized = ctx.input(|i| i.viewport().maximized.unwrap_or(false));
    if maximized {
        return;
    }
    let Some(pos) = ctx.input(|i| i.pointer.hover_pos()) else {
        return;
    };
    let screen = ctx.screen_rect();
    let m = RESIZE_MARGIN;
    let (left, right) = (pos.x - screen.min.x < m, screen.max.x - pos.x < m);
    let (top, bottom) = (pos.y - screen.min.y < m, screen.max.y - pos.y < m);
    let direction = match (left, right, top, bottom) {
        (true, _, true, _) => ResizeDirection::NorthWest,
        (_, true, true, _) => ResizeDirection::NorthEast,
        (true, _, _, true) => ResizeDirection::SouthWest,
        (_, true, _, true) => ResizeDirection::SouthEast,
        (true, _, _, _) => ResizeDirection::West,
        (_, true, _, _) => ResizeDirection::East,
        (_, _, true, _) => ResizeDirection::North,
        (_, _, _, true) => ResizeDirection::South,
        _ => return,
    };
    ctx.set_cursor_icon(match direction {
        ResizeDirection::North | ResizeDirection::South => CursorIcon::ResizeVertical,
        ResizeDirection::East | ResizeDirection::West => CursorIcon::ResizeHorizontal,
        ResizeDirection::NorthWest | ResizeDirection::SouthEast => CursorIcon::ResizeNwSe,
        ResizeDirection::NorthEast | ResizeDirection::SouthWest => CursorIcon::ResizeNeSw,
    });
    if ctx.input(|i| i.pointer.primary_pressed()) {
        ctx.send_viewport_cmd(ViewportCommand::BeginResize(direction));
    }
}
