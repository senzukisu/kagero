//! Kagero's own pixel icons: 16×16 bitmaps with a small fixed palette, drawn
//! with dark outlines and two-tone shading.
//!
//! Bitmap legend: `k` outline, `#` light, `g` gray, `d` dim, `o` ember, `p` flame,
//! `r` rust, `y` glow, `b` sky, `n` navy, `t` mint, `w` wood, `e` blush; anything
//! else is transparent.

use egui::{Color32, Rect, Sense, Stroke, StrokeKind, Vec2};

use crate::editor::ToolKind;
use crate::theme;

/// Width and height of every icon bitmap.
pub const ICON_SIZE: f32 = 16.0;

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
    Play,
    Stop,
    FirstFrame,
    PrevFrame,
    NextFrame,
    Duplicate,
    MoveLeft,
    MoveRight,
    Onion,
    Settings,
    Select,
    Copy,
    FlipH,
    FlipV,
    Rotate,
    Export,
    Keyboard,
    Chat,
    Flask,
    Palette,
    Plug,
    Download,
}

impl Icon {
    pub const ALL: [Icon; 37] = [
        Icon::Pencil,
        Icon::Eraser,
        Icon::Fill,
        Icon::Eyedropper,
        Icon::Line,
        Icon::Rectangle,
        Icon::FilledRectangle,
        Icon::Pan,
        Icon::EyeOpen,
        Icon::EyeClosed,
        Icon::Plus,
        Icon::Trash,
        Icon::ArrowUp,
        Icon::ArrowDown,
        Icon::Swap,
        Icon::Play,
        Icon::Stop,
        Icon::FirstFrame,
        Icon::PrevFrame,
        Icon::NextFrame,
        Icon::Duplicate,
        Icon::MoveLeft,
        Icon::MoveRight,
        Icon::Onion,
        Icon::Settings,
        Icon::Select,
        Icon::Copy,
        Icon::FlipH,
        Icon::FlipV,
        Icon::Rotate,
        Icon::Export,
        Icon::Keyboard,
        Icon::Chat,
        Icon::Flask,
        Icon::Palette,
        Icon::Plug,
        Icon::Download,
    ];

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
            ToolKind::Select => Icon::Select,
        }
    }

    pub fn bitmap(self) -> &'static [&'static str; 16] {
        match self {
            Icon::Pencil => &[
                "............kk..",
                "...........keek.",
                "..........keeek.",
                ".........kgggk..",
                "........kyopk...",
                ".......kyopk....",
                "......kyopk.....",
                ".....kyopk......",
                "....kyopk.......",
                "...kyopk........",
                "..kwwwk.........",
                "..kwwk..........",
                ".kdwk...........",
                ".kdk............",
                ".kk.............",
                "................",
            ],
            Icon::Eraser => &[
                "................",
                "................",
                "........kkkkkk..",
                ".......keeeebbk.",
                "......keeeebbbk.",
                ".....keeeebbbk..",
                "....keeeebbbk...",
                "...k#eeebbbk....",
                "..k#eeebbnk.....",
                "..keeeebnnk.....",
                "..kpeeennk......",
                "..kkppnnk.......",
                "...kkkkk........",
                "................",
                ".ggggggggggggg..",
                "................",
            ],
            Icon::Fill => &[
                "................",
                ".....kkkk.......",
                "....k....k......",
                "...k......k.....",
                "..kkkkkkkkkk....",
                "..kooooooook....",
                "..k#gggggggk.op.",
                "..k#gggggggk.oo.",
                "...k#ggggdk..oo.",
                "...k#ggggdk...p.",
                "...k#ggggdk.....",
                "....k#ggdk......",
                "....kkkkkk......",
                "................",
                "................",
                "................",
            ],
            Icon::Eyedropper => &[
                "...........kkk..",
                "..........kgggk.",
                ".........kggggk.",
                "........kkgggk..",
                ".......kgkkgk...",
                "......k#bkk.....",
                ".....k#bbk......",
                "....k#bbk.......",
                "...k#bbk........",
                "..k#bbk.........",
                "..kbbk..........",
                ".kbnk...........",
                ".kk.............",
                ".b..............",
                ".n..............",
                "................",
            ],
            Icon::Line => &[
                "................",
                ".............kkk",
                ".............kok",
                "............kkkk",
                "...........k#k..",
                "..........k#k...",
                ".........k#k....",
                "........k#k.....",
                ".......k#k......",
                "......k#k.......",
                ".....k#k........",
                "....k#k.........",
                "kkkkk#k.........",
                "koook...........",
                "kkkk............",
                "................",
            ],
            Icon::Rectangle => &[
                "................",
                "................",
                ".kkkkkkkkkkkkkk.",
                ".k############k.",
                ".k#kkkkkkkkkk#k.",
                ".k#k........k#k.",
                ".k#k........k#k.",
                ".k#k........k#k.",
                ".k#k........k#k.",
                ".k#k........k#k.",
                ".k#k........k#k.",
                ".k#kkkkkkkkkk#k.",
                ".k############k.",
                ".kkkkkkkkkkkkkk.",
                "................",
                "................",
            ],
            Icon::FilledRectangle => &[
                "................",
                "................",
                ".kkkkkkkkkkkkkk.",
                ".kyyyyyyyyyyyok.",
                ".kyoooooooooopk.",
                ".kyoooooooooopk.",
                ".kyoooooooooopk.",
                ".kyoooooooooopk.",
                ".kyoooooooooopk.",
                ".kyoooooooooopk.",
                ".kyoooooooooopk.",
                ".kyoooooooooopk.",
                ".kopppppppppprk.",
                ".kkkkkkkkkkkkkk.",
                "................",
                "................",
            ],
            Icon::Pan => &[
                ".......kk.......",
                "......k##k......",
                ".....k####k.....",
                "....kkk##kkk....",
                "......k##k......",
                ".kk...k##k...kk.",
                "k#kkkkk##kkkkk#k",
                "##############kk",
                "k#kkkkk##kkkkk#k",
                ".kk...k##k...kk.",
                "......k##k......",
                "....kkk##kkk....",
                ".....k####k.....",
                "......k##k......",
                ".......kk.......",
                "................",
            ],
            Icon::Select => &[
                "................",
                "................",
                ".##.##.##.##.##.",
                ".#............#.",
                "................",
                ".#............#.",
                ".#............#.",
                "................",
                ".#............#.",
                ".#............#.",
                "................",
                ".#............#.",
                ".##.##.##.##.##.",
                "................",
                "................",
                "................",
            ],
            Icon::EyeOpen => &[
                "................",
                "................",
                "................",
                ".....kkkkkk.....",
                "...kk######kk...",
                "..k###kkkk###k..",
                ".k###kbbbbk###k.",
                "k####kbkkbk####k",
                ".k###kbkkbk###k.",
                "..k###kbbk###k..",
                "...kk##kk###k...",
                ".....kkkkkk.....",
                "................",
                "................",
                "................",
                "................",
            ],
            Icon::EyeClosed => &[
                "................",
                "................",
                "................",
                "................",
                "................",
                "................",
                "g..............g",
                ".g............g.",
                "..gg........gg..",
                "....gggggggg....",
                "...g..g..g..g...",
                "..g...g..g...g..",
                "................",
                "................",
                "................",
                "................",
            ],
            Icon::Plus => &[
                "................",
                "................",
                "......kkkk......",
                "......k##k......",
                "......k##k......",
                "......k##k......",
                "..kkkkk##kkkkk..",
                "..k##########k..",
                "..k##########k..",
                "..kkkkk##kkkkk..",
                "......k##k......",
                "......k##k......",
                "......k##k......",
                "......kkkk......",
                "................",
                "................",
            ],
            Icon::Trash => &[
                "................",
                "......kkkk......",
                ".....k#gg#k.....",
                "..kkkkkkkkkkkk..",
                "..k##########k..",
                "..kkkkkkkkkkkk..",
                "...kgdgdgdgdk...",
                "...kgdgdgdgdk...",
                "...kgdgdgdgdk...",
                "...kgdgdgdgdk...",
                "...kgdgdgdgdk...",
                "...kgdgdgdgdk...",
                "...kgdgdgdgdk...",
                "....kkkkkkkk....",
                "................",
                "................",
            ],
            Icon::ArrowUp => &[
                "................",
                ".......kk.......",
                "......k##k......",
                ".....k####k.....",
                "....k######k....",
                "...k########k...",
                "..kkkkk##kkkkk..",
                "......k##k......",
                "......k##k......",
                "......k##k......",
                "......k##k......",
                "......k##k......",
                "......k##k......",
                "......kkkk......",
                "................",
                "................",
            ],
            Icon::ArrowDown => &[
                "................",
                "................",
                "......kkkk......",
                "......k##k......",
                "......k##k......",
                "......k##k......",
                "......k##k......",
                "......k##k......",
                "......k##k......",
                "..kkkkk##kkkkk..",
                "...k########k...",
                "....k######k....",
                ".....k####k.....",
                "......k##k......",
                ".......kk.......",
                "................",
            ],
            Icon::Swap => &[
                "................",
                "..........k.....",
                "..........kk....",
                ".kkkkkkkkkkok...",
                ".koooooooooook..",
                ".kkkkkkkkkkok...",
                "..........kk....",
                "..........k.....",
                ".....k..........",
                "....kk..........",
                "...k#kkkkkkkkkk.",
                "..k###########k.",
                "...k#kkkkkkkkkk.",
                "....kk..........",
                ".....k..........",
                "................",
            ],
            Icon::Play => &[
                "................",
                "................",
                "....kk..........",
                "....k#k.........",
                "....k##k........",
                "....k###k.......",
                "....k####k......",
                "....k#####k.....",
                "....k####gk.....",
                "....k###gk......",
                "....k##gk.......",
                "....k#gk........",
                "....kgk.........",
                "....kk..........",
                "................",
                "................",
            ],
            Icon::Stop => &[
                "................",
                "................",
                "................",
                "...kkkkkkkkkk...",
                "...k########k...",
                "...k########k...",
                "...k########k...",
                "...k########k...",
                "...k########k...",
                "...k########k...",
                "...k#gggggggk...",
                "...kkkkkkkkkk...",
                "................",
                "................",
                "................",
                "................",
            ],
            Icon::FirstFrame => &[
                "................",
                "................",
                "................",
                "..kkk....kk.....",
                "..k#k...k#k.....",
                "..k#k..k##k.....",
                "..k#k.k###k.....",
                "..k#kk####k.....",
                "..k#k.k###k.....",
                "..k#k..k##k.....",
                "..k#k...k#k.....",
                "..kkk....kk.....",
                "................",
                "................",
                "................",
                "................",
            ],
            Icon::PrevFrame => &[
                "................",
                "................",
                "................",
                ".........kk.....",
                "........k#k.....",
                ".......k##k.....",
                "......k###k.....",
                ".....k####k.....",
                "......k###k.....",
                ".......k##k.....",
                "........k#k.....",
                ".........kk.....",
                "................",
                "................",
                "................",
                "................",
            ],
            Icon::NextFrame => &[
                "................",
                "................",
                "................",
                ".....kk.........",
                ".....k#k........",
                ".....k##k.......",
                ".....k###k......",
                ".....k####k.....",
                ".....k###k......",
                ".....k##k.......",
                ".....k#k........",
                ".....kk.........",
                "................",
                "................",
                "................",
                "................",
            ],
            Icon::Duplicate => &[
                "................",
                ".kkkkkkkkk......",
                ".k#######k......",
                ".k#ggggg#k......",
                ".k#g.kkkkkkkkk..",
                ".k#g.k#######k..",
                ".k#g.k#ooooo#k..",
                ".k#g.k#ooooo#k..",
                ".k###k#ooooo#k..",
                ".kkkkk#ooooo#k..",
                ".....k#ooooo#k..",
                ".....k#ooooo#k..",
                ".....k#######k..",
                ".....kkkkkkkkk..",
                "................",
                "................",
            ],
            Icon::MoveLeft => &[
                "................",
                "................",
                "................",
                ".........kkkkkkk",
                ".........k#####k",
                "...o.....k#####k",
                "..oo.....k#ggg#k",
                ".ooooooo.k#####k",
                ".ooooooo.k#####k",
                "..oo.....k#ggg#k",
                "...o.....k#####k",
                ".........k#####k",
                ".........kkkkkkk",
                "................",
                "................",
                "................",
            ],
            Icon::MoveRight => &[
                "................",
                "................",
                "................",
                "kkkkkkk.........",
                "k#####k.........",
                "k#####k.....o...",
                "k#ggg#k.....oo..",
                "k#####k.ooooooo.",
                "k#####k.ooooooo.",
                "k#ggg#k.....oo..",
                "k#####k.....o...",
                "k#####k.........",
                "kkkkkkk.........",
                "................",
                "................",
                "................",
            ],
            Icon::Onion => &[
                "................",
                "ppppppppp.......",
                "p.......p.......",
                "p..kkkkkkkkkk...",
                "p..k########k...",
                "p..k########k...",
                "p..k##oooo##kttt",
                "p..k#o####o#k..t",
                "p..k#o####o#k..t",
                "pppk##oooo##k..t",
                "...k########k..t",
                "...k########k..t",
                "...kkkkkkkkkk..t",
                ".......t.......t",
                ".......ttttttttt",
                "................",
            ],
            Icon::Settings => &[
                "................",
                "......kkkk......",
                "..kk..k##k..kk..",
                ".k##kk####kk##k.",
                ".k############k.",
                "..k####kk####k..",
                ".kk###kddk###kk.",
                "k####kd..dk####k",
                "k####kd..dk####k",
                ".kk###kddk###kk.",
                "..k####kk####k..",
                ".k############k.",
                ".k##kk####kk##k.",
                "..kk..k##k..kk..",
                "......kkkk......",
                "................",
            ],
            Icon::Copy => &[
                "................",
                ".kkkkkkkk.......",
                ".k######k.......",
                ".k#gggg#k.......",
                ".k#....kkkkkkk..",
                ".k#g...k#####k..",
                ".k#g...k#bbb#k..",
                ".k#g...k#####k..",
                ".k#####k#bbb#k..",
                ".kkkkkkk#####k..",
                ".......k#bbb#k..",
                ".......k#####k..",
                ".......kkkkkkk..",
                "................",
                "................",
                "................",
            ],
            Icon::FlipH => &[
                "................",
                ".......kk.......",
                "..k....k#....k..",
                "..kk...kk...kk..",
                "..kok..k#..kgk..",
                "..kook.kk.kggk..",
                "..koook.#kgggk..",
                "..koooo.kggggk..",
                "..koook.#kgggk..",
                "..kook.kk.kggk..",
                "..kok..k#..kgk..",
                "..kk...kk...kk..",
                "..k....k#....k..",
                ".......kk.......",
                "................",
                "................",
            ],
            Icon::FlipV => &[
                "................",
                "................",
                "..kkkkkkkkkkkk..",
                "...kooooooook...",
                "....koooooook...",
                ".....kooook.....",
                "......kook......",
                ".kk.kk.kk.kk.kk.",
                ".#k#k#.##.#k#k#.",
                "......kggk......",
                ".....kggggk.....",
                "....kggggggk....",
                "...kggggggggk...",
                "..kkkkkkkkkkkk..",
                "................",
                "................",
            ],
            Icon::Rotate => &[
                "................",
                "........kk......",
                "....kkkkk#k.....",
                "...k#######k....",
                "..k#kkkkk#k.....",
                ".k#k....kk......",
                ".k#k............",
                "k#k.........kkk.",
                "k#k.........k#k.",
                "k#k.........k#k.",
                ".k#k.......k#k..",
                ".k#kk.....kk#k..",
                "..k##kkkkk##k...",
                "...kk#####kk....",
                ".....kkkkk......",
                "................",
            ],
            Icon::Export => &[
                "................",
                ".......kk.......",
                "......kook......",
                ".....koooopk....",
                "....koooooopk...",
                "...kkkkoopkkkk..",
                "......koopk.....",
                "......koopk.....",
                "......koopk.....",
                "..kkk.kkkkk.kkk.",
                "..k#k.......k#k.",
                "..k#kkkkkkkkk#k.",
                "..k###########k.",
                "..kkkkkkkkkkkkk.",
                "................",
                "................",
            ],
            Icon::Keyboard => &[
                "................",
                "................",
                "................",
                ".kkkkkkkkkkkkkk.",
                ".kddddddddddddk.",
                ".kd#d#d#d#d#ddk.",
                ".kddddddddddddk.",
                ".kdd#d#d#d#d#dk.",
                ".kddddddddddddk.",
                ".kd#d######d#dk.",
                ".kddddddddddddk.",
                ".kkkkkkkkkkkkkk.",
                "................",
                "................",
                "................",
                "................",
            ],
            Icon::Chat => &[
                "................",
                "................",
                "..kkkkkkkkkkkk..",
                ".kbbbbbbbbbbbbk.",
                ".kbbbbbbbbbbbbk.",
                ".kbb##bbbb##bbk.",
                ".kbb##bbbb##bbk.",
                ".kbbbbbbbbbbbbk.",
                ".kbbb#bbbb#bbbk.",
                ".kbbbb####bbbbk.",
                ".knbbbbbbbbbbnk.",
                "..kknnnnnnnnkk..",
                "....knk.kkkk....",
                "....kk..........",
                "................",
                "................",
            ],
            Icon::Flask => &[
                "................",
                ".....kkkkkk.....",
                "......k##k......",
                "......k##k......",
                "......k##k......",
                ".....k####k.....",
                "....k######k....",
                "....k##tt##k....",
                "...k#tttttt#k...",
                "..ktttyttttttk..",
                "..kttttttttytk..",
                "..kttttttttttk..",
                "...kkkkkkkkkk...",
                "................",
                "................",
                "................",
            ],
            Icon::Palette => &[
                "................",
                "................",
                "....kkkkkkk.....",
                "...kwwwwwwwkk...",
                "..kwwbbwwwwwwk..",
                ".kwwbbwwwwppwwk.",
                ".kwwwwwwwwppwwk.",
                ".kwwwwkkwwwwwwk.",
                ".kwttwk.kwwwwwk.",
                ".kwttwk.kwwyywk.",
                ".kwwwwwkwwwyywk.",
                "..kwwwwwwwwwwk..",
                "...kkwwwwwkkk...",
                ".....kkkkk......",
                "................",
                "................",
            ],
            Icon::Plug => &[
                "................",
                "....kkk..kkk....",
                "....kgk..kgk....",
                "....kgk..kgk....",
                "..kkkkkkkkkkkk..",
                "..k##########k..",
                "..k#gggggggg#k..",
                "..k#gggggggg#k..",
                "...k#gggggg#k...",
                "....k#gggg#k....",
                ".....kk##kk.....",
                "......k##k......",
                "......k##k......",
                ".......oo.......",
                ".......oo.......",
                "................",
            ],
            Icon::Download => &[
                "................",
                "......koopk.....",
                "......koopk.....",
                "......koopk.....",
                "...kkkkoopkkkk..",
                "....koooooopk...",
                ".....koooopk....",
                "......kook......",
                ".......kk.......",
                "..kkk.kkkkk.kkk.",
                "..k#k.......k#k.",
                "..k#kkkkkkkkk#k.",
                "..k###########k.",
                "..kkkkkkkkkkkkk.",
                "................",
                "................",
            ],
        }
    }
}

/// How an icon is colored.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Tint {
    /// The icon's own colors.
    Normal,
    /// Faded gray, for disabled buttons.
    Disabled,
}

/// The color of one bitmap character, or `None` for transparent.
pub fn palette_color(ch: u8) -> Option<Color32> {
    Some(match ch {
        b'k' => Color32::from_rgb(0x0E, 0x0F, 0x13),
        b'#' => Color32::from_rgb(0xF0, 0xEE, 0xE8),
        b'g' => Color32::from_rgb(0xA5, 0xA9, 0xB5),
        b'd' => Color32::from_rgb(0x4A, 0x4F, 0x5C),
        b'o' => Color32::from_rgb(0xFF, 0x9D, 0x42),
        b'p' => Color32::from_rgb(0xF0, 0x6B, 0x35),
        b'r' => Color32::from_rgb(0xA5, 0x3A, 0x26),
        b'y' => Color32::from_rgb(0xFF, 0xE7, 0xA8),
        b'b' => Color32::from_rgb(0x5F, 0xA0, 0xE0),
        b'n' => Color32::from_rgb(0x37, 0x67, 0xA8),
        b't' => Color32::from_rgb(0x75, 0xC7, 0xB5),
        b'w' => Color32::from_rgb(0xC8, 0x8B, 0x5A),
        b'e' => Color32::from_rgb(0xE0, 0x7A, 0xB4),
        _ => return None,
    })
}

fn tinted(c: Color32, tint: Tint) -> Color32 {
    match tint {
        Tint::Normal => c,
        Tint::Disabled => {
            let l = (u16::from(c.r()) * 3 + u16::from(c.g()) * 6 + u16::from(c.b())) / 10;
            let v = (l / 3 + 40) as u8;
            Color32::from_rgb(v, v, v + 4)
        }
    }
}

/// Paints a 16×16 icon centered in `rect`, scaled by whole pixels.
pub fn paint_icon(painter: &egui::Painter, rect: Rect, icon: Icon, tint: Tint) {
    paint_bitmap(painter, rect, icon.bitmap(), tint);
}

/// Paints any bitmap (rows of equal length) centered in `rect`, at the largest whole
/// scale that fits, on whole physical pixels.
pub fn paint_bitmap(painter: &egui::Painter, rect: Rect, rows: &[&str], tint: Tint) {
    let h = rows.len().max(1) as f32;
    let w = rows.iter().map(|r| r.len()).max().unwrap_or(1).max(1) as f32;
    let ppp = painter.ctx().pixels_per_point();
    // Scale in device pixels, so every bitmap pixel is a whole number of them.
    let device = ((rect.width() / w).min(rect.height() / h) * ppp)
        .floor()
        .max(1.0);
    let scale = device / ppp;
    let origin = ((rect.center() - Vec2::new(w, h) * scale / 2.0) * ppp).round() / ppp;
    // One plain mesh, no anti-aliasing: egui's feathered `rect_filled` collapses
    // rectangles that are a single pixel wide.
    let mut mesh = egui::Mesh::default();
    for (y, row) in rows.iter().enumerate() {
        for (x, ch) in row.bytes().enumerate() {
            let Some(color) = palette_color(ch) else {
                continue;
            };
            let min = origin + Vec2::new(x as f32, y as f32) * scale;
            mesh.add_colored_rect(
                Rect::from_min_size(min, Vec2::splat(scale)),
                tinted(color, tint),
            );
        }
    }
    painter.add(egui::Shape::mesh(mesh));
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
            let Some(c) = palette_color(ch) else {
                continue;
            };
            let i = ((y * size + x) * 4) as usize;
            rgba[i..i + 4].copy_from_slice(&[c.r(), c.g(), c.b(), 255]);
        }
    }
    (rgba, size, size)
}

/// A 1-pixel bevel inside `rect`: light top-left and dark bottom-right when
/// `raised`, the other way round when pressed in.
pub fn bevel(painter: &egui::Painter, rect: Rect, raised: bool) {
    let px = 1.0 / painter.ctx().pixels_per_point();
    let (tl, br) = if raised {
        (theme::BEVEL_LIGHT, theme::BEVEL_DARK)
    } else {
        (theme::BEVEL_DARK, theme::BEVEL_LIGHT)
    };
    let r = rect;
    let mut mesh = egui::Mesh::default();
    mesh.add_colored_rect(
        Rect::from_min_max(r.min, egui::pos2(r.max.x, r.min.y + px)),
        tl,
    );
    mesh.add_colored_rect(
        Rect::from_min_max(r.min, egui::pos2(r.min.x + px, r.max.y)),
        tl,
    );
    mesh.add_colored_rect(
        Rect::from_min_max(egui::pos2(r.min.x, r.max.y - px), r.max),
        br,
    );
    mesh.add_colored_rect(
        Rect::from_min_max(egui::pos2(r.max.x - px, r.min.y), r.max),
        br,
    );
    painter.add(egui::Shape::mesh(mesh));
}

/// A square icon button. `selected` shows it pressed in with the ember color.
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
        let pressed = enabled && response.is_pointer_button_down_on();
        let hovered = enabled && response.hovered();
        if selected || pressed {
            painter.rect_filled(rect, 0.0, theme::EMBER_DEEP);
            bevel(painter, rect, false);
            if selected {
                painter.rect_stroke(
                    rect,
                    0.0,
                    Stroke::new(1.0_f32, theme::ACCENT),
                    StrokeKind::Inside,
                );
            }
        } else if hovered {
            painter.rect_filled(rect, 0.0, theme::SURFACE_HOVER);
            bevel(painter, rect, true);
        }
        let tint = if enabled {
            Tint::Normal
        } else {
            Tint::Disabled
        };
        // 2× icons from 36 px buttons up, 1× below; pressed icons sink by a pixel.
        let scale = ((size - 4.0) / ICON_SIZE).floor().max(1.0);
        let shift = if pressed && !selected { 1.0 } else { 0.0 };
        let inner = Rect::from_center_size(
            rect.center() + Vec2::splat(shift),
            Vec2::splat(ICON_SIZE * scale),
        );
        paint_icon(painter, inner, icon, tint);
    }
    response.on_hover_text(tooltip)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bitmaps_are_16_by_16_with_known_colors() {
        for icon in Icon::ALL {
            let rows = icon.bitmap();
            for row in rows {
                assert_eq!(row.len(), 16, "{icon:?}: {row}");
                for ch in row.bytes() {
                    assert!(ch == b'.' || palette_color(ch).is_some(), "{icon:?}: {row}");
                }
            }
            assert!(
                rows.iter().any(|r| r.bytes().any(|c| c != b'.')),
                "{icon:?} is empty"
            );
        }
    }

    #[test]
    fn disabled_icons_are_gray() {
        let c = tinted(Color32::from_rgb(0xFF, 0x9D, 0x42), Tint::Disabled);
        assert!(c.r().abs_diff(c.g()) <= 4 && c.g().abs_diff(c.b()) <= 4);
    }
}
