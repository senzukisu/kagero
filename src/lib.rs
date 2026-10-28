//! Kagero — an independent pixel-art editor by Onibi Lab.
//!
//! The crate is split into four layers:
//! - [`core`]: the pixel engine (RGBA buffers, layers, compositing, raster algorithms);
//! - [`editor`]: the application layer (document session, tools, undoable commands, history);
//! - [`file_io`]: loading, saving, exporting and validating files;
//! - [`ui`] + [`app`]: egui windows, panels, canvas and input handling.

pub mod app;
pub mod config;
pub mod core;
pub mod editor;
pub mod export;
pub mod file_io;
pub mod integrations;
pub mod release;
pub mod services;
pub mod theme;
pub mod ui;

/// Human-readable application name.
pub const APP_NAME: &str = "Kagero";
/// Studio that develops Kagero.
pub const STUDIO_NAME: &str = "Onibi Lab";
/// Application version, taken from Cargo.toml.
pub const APP_VERSION: &str = env!("CARGO_PKG_VERSION");
/// Release channel shown next to the version.
pub const APP_CHANNEL: &str = "Beta";
