//! egui user interface: screens, panels, canvas and dialogs.
//!
//! UI code never mutates documents itself. Panels return [`Command`]s that the app
//! executes through the editor layer.

pub mod animation_preview;
pub mod canvas;
pub mod dialogs;
pub mod home;
pub mod icons;
pub mod layers;
pub mod menu;
pub mod palette;
pub mod settings;
pub mod shortcuts;
pub mod status;
pub mod timeline;
pub mod tool_options;
pub mod toolbar;

use std::path::PathBuf;

use crate::editor::ToolKind;

/// Every user action that the app knows how to perform.
#[derive(Debug, Clone, PartialEq)]
pub enum Command {
    NewDocument,
    Open,
    OpenPath(PathBuf),
    RemoveRecent(PathBuf),
    Save,
    SaveAs,
    ExportPng,
    CloseDocument,
    Exit,
    Undo,
    Redo,
    SetTool(ToolKind),
    SwapColors,
    AddPaletteColor,
    RemovePaletteColor(usize),
    ResetPalette,
    ToggleGrid,
    CenterCanvas,
    FitCanvas,
    ZoomIn,
    ZoomOut,
    ZoomActual,
    AddLayer,
    DeleteLayer,
    LayerUp,
    LayerDown,
    SelectLayer(usize),
    ToggleLayerVisibility(usize),
    BeginRenameLayer(usize),
    RenameLayer(usize, String),
    About,
    // Animation
    TogglePlayback,
    StopPlayback,
    FirstFrame,
    PrevFrame,
    NextFrame,
    SelectFrame(usize),
    AddFrame,
    DuplicateFrame,
    /// Asks for confirmation when the frame has content.
    DeleteFrame(usize),
    DeleteFrameConfirmed(usize),
    MoveFrameLeft(usize),
    MoveFrameRight(usize),
    SetFrameDuration(usize, u32),
    ToggleOnionSkin,
    ExportAllFrames,
    // Workspace and services
    ToggleTimeline,
    ToggleSidePanel,
    OpenSettings(settings::SettingsTab),
    CheckForUpdates,
    /// Persist settings changed directly by a panel widget.
    SettingsChanged,
}
