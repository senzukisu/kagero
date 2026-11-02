//! Opening, saving and exporting files.

use super::*;

impl KageroApp {
    /// Installs a new document, keeping the tool and colors of the previous one.
    pub(super) fn set_document(&mut self, doc: Document, path: Option<PathBuf>, name: String) {
        self.player.reset();
        // The previous document was saved or deliberately discarded by now.
        self.recovery.discard();
        self.last_autosave = Instant::now();
        let mut editor = Editor::new(doc, path, name);
        if let Some(old) = &self.editor {
            editor.tool = old.tool;
            editor.settings = old.settings;
            editor.primary = old.primary;
            editor.secondary = old.secondary;
            editor.clipboard = old.clipboard.clone();
        }
        self.editor = Some(editor);
        self.rename = None;
        self.canvas.reset();
        self.timeline.clear();
    }

    pub(super) fn create_document(&mut self, state: &NewDocumentState) {
        match Document::new(state.width, state.height, state.background.color()) {
            Ok(doc) => {
                self.set_document(doc, None, "Untitled".to_owned());
                self.config.new_width = state.width;
                self.config.new_height = state.height;
                self.save_config();
                self.info(format!("New image {}×{}", state.width, state.height));
            }
            Err(e) => self.error_dialog("Could not create the image", &e.into()),
        }
    }

    pub(super) fn open_path(&mut self, path: &Path) {
        let ext = path
            .extension()
            .map(|e| e.to_string_lossy().to_ascii_lowercase())
            .unwrap_or_default();
        let result = match ext.as_str() {
            file_io::KGR_EXTENSION => {
                file_io::load_kgr(path).map(|d| (d, Some(path.to_path_buf())))
            }
            "png" => file_io::import_png(path).map(|d| (d, None)),
            _ => Err(FileError::Invalid(
                "Kagero opens .kgr projects and .png images".to_owned(),
            )),
        };
        let file_name = path.file_name().map_or_else(
            || path.display().to_string(),
            |n| n.to_string_lossy().into_owned(),
        );
        match result {
            Ok((doc, save_path)) => {
                let imported = save_path.is_none();
                let frames = doc.frame_count();
                self.set_document(doc, save_path, display_name(path));
                self.config.add_recent(path);
                self.save_config();
                if imported {
                    self.info(format!(
                        "Imported {file_name} — use Save to keep it as a .kgr project"
                    ));
                } else if frames > 1 {
                    self.success(format!("Opened {file_name} · {frames} frames"));
                } else {
                    self.success(format!("Opened {file_name}"));
                }
                tracing::info!("opened {}", path.display());
            }
            Err(e) => {
                tracing::error!("could not open {}: {e}", path.display());
                self.error_dialog(format!("Could not open “{file_name}”"), &e);
            }
        }
    }

    /// Saves to the current path, or asks for one. Returns `true` on success.
    pub(super) fn save(&mut self) -> bool {
        let Some(editor) = &mut self.editor else {
            return false;
        };
        editor.end_stroke();
        match editor.path().map(Path::to_path_buf) {
            Some(path) => self.save_to(path),
            None => self.save_as(),
        }
    }

    pub(super) fn save_as(&mut self) -> bool {
        let Some(editor) = &mut self.editor else {
            return false;
        };
        editor.end_stroke();
        let name = editor.name().to_owned();
        match native::pick_save_project(self.start_dir().as_deref(), &name) {
            Some(path) => self.save_to(path),
            None => false,
        }
    }

    pub(super) fn save_to(&mut self, path: PathBuf) -> bool {
        let Some(editor) = &mut self.editor else {
            return false;
        };
        match file_io::save_kgr(editor.document(), &path) {
            Ok(()) => {
                editor.mark_saved(path.clone());
                self.recovery.discard();
                self.config.add_recent(&path);
                self.save_config();
                self.success(format!("Saved {}", path.display()));
                tracing::info!("saved {}", path.display());
                true
            }
            Err(e) => {
                tracing::error!("could not save {}: {e}", path.display());
                self.error_dialog("The project was not saved", &e);
                false
            }
        }
    }

    pub(super) fn export_png(&mut self) {
        let Some(editor) = &mut self.editor else {
            return;
        };
        editor.end_stroke();
        let name = editor.name().to_owned();
        let Some(path) = native::pick_export_png(self.start_dir().as_deref(), &name) else {
            return;
        };
        let Some(editor) = &self.editor else {
            return;
        };
        let doc = editor.document();
        let png = crate::export::ExportSettings {
            kind: crate::export::ExportKind::Image,
            still_format: crate::export::StillFormat::Png,
            ..self.config.export.clone()
        };
        let scale = png.effective_scale(doc.width(), doc.height());
        let result = (|| -> Result<(u32, u32), String> {
            let merged = doc.composite().map_err(|e| e.to_string())?;
            let img = image::RgbaImage::from_raw(doc.width(), doc.height(), merged.into_bytes())
                .ok_or("the image could not be prepared")?;
            let img =
                crate::export::render::scale_nearest(&img, scale).map_err(|e| e.to_string())?;
            let size = img.dimensions();
            let mut bytes = Vec::new();
            crate::export::still::encode_png(&img, &mut bytes).map_err(|e| e.to_string())?;
            file_io::write_atomic(&path, |w| {
                std::io::Write::write_all(w, &bytes)?;
                Ok(())
            })
            .map_err(|e| e.to_string())?;
            Ok(size)
        })();
        match result {
            Ok((w, h)) if scale > 1 => self.success(format!(
                "Exported {} at {w}×{h} (×{scale}; change the size in File → Export…)",
                path.display()
            )),
            Ok(_) => self.success(format!("Exported {}", path.display())),
            Err(e) => {
                tracing::error!("could not export {}: {e}", path.display());
                self.status_error(format!("The PNG was not exported: {e}"));
            }
        }
    }

    pub(super) fn import_palette(&mut self) {
        let Some(path) = rfd::FileDialog::new()
            .set_title("Import Palette")
            .add_filter("Palettes", &["gpl", "hex", "txt"])
            .pick_file()
        else {
            return;
        };
        match palette_io::load_palette(&path) {
            Ok(palette) => {
                let name = palette.name.clone();
                if self.config.add_palette(palette) {
                    self.schedule_config_save();
                    self.success(format!("Imported palette “{name}”"));
                } else {
                    self.status_error("Too many palettes — delete one first");
                }
            }
            Err(e) => self.error_dialog("The palette was not imported", &e),
        }
    }

    pub(super) fn export_palette(&mut self) {
        let palette = self.config.active_palette().clone();
        let Some(path) = rfd::FileDialog::new()
            .set_title("Export Palette")
            .add_filter("GIMP palette", &["gpl"])
            .add_filter("HEX list", &["hex"])
            .set_file_name(format!(
                "{}.gpl",
                crate::export::sequence::sanitize_stem(&palette.name)
            ))
            .save_file()
        else {
            return;
        };
        match palette_io::save_palette(&palette, &path) {
            Ok(()) => self.success(format!("Saved {}", path.display())),
            Err(e) => self.error_dialog("The palette was not saved", &e),
        }
    }
}
