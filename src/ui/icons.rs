//! Kagero's own pixel icons, drawn from tiny bitmaps.
//!
//! Bitmap legend: `#` = foreground, `o` = accent, `y` = highlight, `p` = deep accent,
//! anything else = transparent.

use egui::{Color32, Rect, Sense, Vec2};

use crate::editor::ToolKind;
use crate::theme;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Icon {
    Pencil,
    Eraser,
    Fill,
    Eyedropper,
    Line,
    Rectangle,
    FilledRectangle,
    Pan,
    EyeOpen,
    EyeClosed,
    Plus,
    Trash,
    ArrowUp,
    ArrowDown,
    Swap,
}

impl Icon {
    pub fn for_tool(tool: ToolKind) -> Icon {
        match tool {
            ToolKind::Pencil => Icon::Pencil,
            ToolKind::Eraser => Icon::Eraser,
            ToolKind::Fill => Icon::Fill,
            ToolKind::Eyedropper => Icon::Eyedropper,
            ToolKind::Line => Icon::Line,
            ToolKind::Rectangle => Icon::Rectangle,
            ToolKind::FilledRectangle => Icon::FilledRectangle,
            ToolKind::Pan => Icon::Pan,
        }
    }

    fn bitmap(self) -> &'static [&'static str; 12] {
        match self {
            Icon::Pencil => &[
                "..........##",
                ".........#oo",
                "........#oo#",
                ".......#oo#.",
                "......#oo#..",
                ".....#oo#...",
                "....#oo#....",
                "...#oo#.....",
                "..#oo#......",
                "..##o#......",
                ".###........",
                ".#..........",
            ],
            Icon::Eraser => &[
                "............",
                "............",
                ".....#######",
                "....#oooo#.#",
                "...#oooo#..#",
                "..#oooo#..#.",
                ".#oooo#..#..",
                ".#ooo#..#...",
                ".#####.#....",
                "..######....",
                "............",
                "############",
            ],
            Icon::Fill => &[
                "....##......",
                "...#..#.....",
                "...#..#.....",
                "..########..",
                ".#########o.",
                ".#########oo",
                ".#########oo",
                "..#######.oo",
                "...#####..o.",
                "....###.....",
                "............",
                "............",
            ],
            Icon::Eyedropper => &[
                ".........##.",
                "........####",
                ".......#####",
                "......####..",
                ".....#.##...",
                "....#o#.....",
                "...#o#......",
                "..#o#.......",
                ".#o#........",
                ".o#.........",
                "o...........",
                "............",
            ],
            Icon::Line => &[
                "..........oo",
                "..........oo",
                ".........#..",
                "........#...",
                ".......#....",
                "......#.....",
                ".....#......",
                "....#.......",
                "...#........",
                "..#.........",
                "oo..........",
                "oo..........",
            ],
            Icon::Rectangle => &[
                "............",
                ".##########.",
                ".#........#.",
                ".#........#.",
                ".#........#.",
                ".#........#.",
                ".#........#.",
                ".#........#.",
                ".#........#.",
                ".##########.",
                "............",
                "............",
            ],
            Icon::FilledRectangle => &[
                "............",
                ".##########.",
                ".#oooooooo#.",
                ".#oooooooo#.",
                ".#oooooooo#.",
                ".#oooooooo#.",
                ".#oooooooo#.",
                ".#oooooooo#.",
                ".#oooooooo#.",
                ".##########.",
                "............",
                "............",
            ],
            Icon::Pan => &[
                ".....##.....",
                "....####....",
                "...######...",
                ".....##.....",
                ".#...##...#.",
                "############",
                "############",
                ".#...##...#.",
                ".....##.....",
                "...######...",
                "....####....",
                ".....##.....",
            ],
            Icon::EyeOpen => &[
                "............",
                "............",
                "............",
                "....####....",
                "..##....##..",
                ".#...oo...#.",
                "#...oooo...#",
                ".#...oo...#.",
                "..##....##..",
                "....####....",
                "............",
                "............",
            ],
            Icon::EyeClosed => &[
                "............",
                "............",
                "............",
                "............",
                "............",
                "#..........#",
                ".#........#.",
                "..##....##..",
                "....####....",
                "...#..#..#..",
                "............",
                "............",
            ],
            Icon::Plus => &[
                "............",
                ".....##.....",
                ".....##.....",
                ".....##.....",
                ".....##.....",
                ".##########.",
                ".##########.",
                ".....##.....",
                ".....##.....",
                ".....##.....",
                ".....##.....",
                "............",
            ],
            Icon::Trash => &[
                "....####....",
                "############",
                "............",
                ".##########.",
                ".#.#.##.#.#.",
                ".#.#.##.#.#.",
                ".#.#.##.#.#.",
                ".#.#.##.#.#.",
                ".#.#.##.#.#.",
                ".#.#.##.#.#.",
                ".##########.",
                "............",
            ],
            Icon::ArrowUp => &[
                "............",
                ".....##.....",
                "....####....",
                "...######...",
                "..########..",
                ".##########.",
                ".....##.....",
                ".....##.....",
                ".....##.....",
                ".....##.....",
                ".....##.....",
                "............",
            ],
            Icon::ArrowDown => &[
                "............",
                ".....##.....",
                ".....##.....",
                ".....##.....",
                ".....##.....",
                ".....##.....",
                ".##########.",
                "..########..",
                "...######...",
                "....####....",
                ".....##.....",
                "............",
            ],
            Icon::Swap => &[
                "............",
                "...#........",
                "..##........",
                ".##########.",
                "..##.......#",
                "...#.......#",
                "#.......#...",
                "#.......##..",
                ".##########.",
                "........##..",
                "........#...",
                "............",
            ],
        }
    }
}

/// Paints a 12×12 bitmap icon centered in `rect`, scaled by whole pixels.
pub fn paint_icon(painter: &egui::Painter, rect: Rect, icon: Icon, fg: Color32, accent: Color32) {
    paint_bitmap(painter, rect, icon.bitmap(), fg, accent);
}

/// Paints any bitmap (rows of equal length) centered in `rect`.
pub fn paint_bitmap(
    painter: &egui::Painter,
    rect: Rect,
    rows: &[&str],
    fg: Color32,
    accent: Color32,
) {
    let h = rows.len().max(1) as f32;
    let w = rows.iter().map(|r| r.len()).max().unwrap_or(1).max(1) as f32;
    let scale = (rect.width() / w).min(rect.height() / h).floor().max(1.0);
    let origin = (rect.center() - Vec2::new(w, h) * scale / 2.0).round();
    for (y, row) in rows.iter().enumerate() {
        for (x, ch) in row.bytes().enumerate() {
            let color = match ch {
                b'#' => fg,
                b'o' => accent,
                b'y' => Color32::from_rgb(0xFF, 0xE7, 0xA8),
                b'p' => theme::ACCENT_PRESSED,
                _ => continue,
            };
            let min = origin + Vec2::new(x as f32, y as f32) * scale;
            painter.rect_filled(Rect::from_min_size(min, Vec2::splat(scale)), 0.0, color);
        }
    }
}

/// The Kagero flame mark (16×16): heat shimmer over an ember.
pub const LOGO: [&str; 16] = [
    "................",
    ".......o........",
    "......oo........",
    "......ooo.......",
    ".....ooyo...o...",
    "....ooyyo..oo...",
    "...ooyyyoo.oo...",
    "...oyyyyyoooo...",
    "..ooyyyyyyooo...",
    "..oyyyyyyyyoop..",
    "..oyyyyyyyyyop..",
    "..poyyyyyyyyop..",
    "...poyyyyyyop...",
    "....ppooooopp...",
    "......pppp......",
    "................",
];

/// RGBA pixels of the logo scaled up by `scale`, for the window icon.
pub fn logo_rgba(scale: u32) -> (Vec<u8>, u32, u32) {
    let size = 16 * scale;
    let mut rgba = vec![0u8; (size * size * 4) as usize];
    for y in 0..size {
        for x in 0..size {
            let ch = LOGO[(y / scale) as usize]
                .as_bytes()
                .get((x / scale) as usize)
                .copied()
                .unwrap_or(b'.');
            let c = match ch {
                b'o' => theme::ACCENT,
                b'y' => Color32::from_rgb(0xFF, 0xE7, 0xA8),
                b'p' => theme::ACCENT_PRESSED,
                _ => continue,
            };
            let i = ((y * size + x) * 4) as usize;
            rgba[i..i + 4].copy_from_slice(&[c.r(), c.g(), c.b(), 255]);
        }
    }
    (rgba, size, size)
}

/// A square icon button. `selected` draws it with the accent background.
pub fn icon_button(
    ui: &mut egui::Ui,
    icon: Icon,
    size: f32,
    selected: bool,
    enabled: bool,
    tooltip: &str,
) -> egui::Response {
    let sense = if enabled {
        Sense::click()
    } else {
        Sense::hover()
    };
    let (rect, response) = ui.allocate_exact_size(Vec2::splat(size), sense);
    if ui.is_rect_visible(rect) {
        let painter = ui.painter();
        let bg = if selected {
            theme::ACCENT
        } else if enabled && response.is_pointer_button_down_on() {
            theme::ACCENT_PRESSED
        } else if enabled && response.hovered() {
            theme::SURFACE_HOVER
        } else {
            Color32::TRANSPARENT
        };
        painter.rect_filled(rect, theme::RADIUS as f32, bg);
        let (fg, accent) = if selected {
            (theme::ON_ACCENT, theme::TEXT_PRIMARY)
        } else if enabled {
            (theme::TEXT_PRIMARY, theme::ACCENT)
        } else {
            (
                theme::TEXT_SECONDARY.linear_multiply(0.4),
                theme::TEXT_SECONDARY.linear_multiply(0.4),
            )
        };
        // Whole-pixel scale so icons stay crisp: 2× for buttons of 28 px and up.
        let scale = ((size - 4.0) / 12.0).floor().max(1.0);
        let inner = Rect::from_center_size(rect.center(), Vec2::splat(12.0 * scale));
        paint_icon(painter, inner, icon, fg, accent);
    }
    response.on_hover_text(tooltip)
}
