//! What happens when a dialog is answered.

use super::*;

impl KageroApp {
    pub(super) fn show_dialogs(&mut self, ctx: &egui::Context) {
        let Some(dialog) = self.dialog.take() else {
            return;
        };
        match dialog {
            Dialog::NewDocument(mut state) => match dialogs::new_document(ctx, &mut state) {
                NewDocumentResult::Open => self.dialog = Some(Dialog::NewDocument(state)),
                NewDocumentResult::Create => self.create_document(&state),
                NewDocumentResult::Cancel => {}
            },
            Dialog::Unsaved(pending) => {
                let name = self
                    .editor
                    .as_ref()
                    .map_or_else(|| "Untitled".to_owned(), |e| e.name().to_owned());
                match dialogs::unsaved_changes(ctx, &name) {
                    UnsavedChoice::Pending => self.dialog = Some(Dialog::Unsaved(pending)),
                    UnsavedChoice::Save => {
                        if self.save() {
                            self.perform(ctx, pending);
                        }
                    }
                    UnsavedChoice::Discard => self.perform(ctx, pending),
                    UnsavedChoice::Cancel => {}
                }
            }
            Dialog::Error { title, message } => {
                if !dialogs::error(ctx, &title, &message) {
                    self.dialog = Some(Dialog::Error { title, message });
                }
            }
            Dialog::ConvertPalette(mut state) => {
                let palette = self.config.active_palette().clone();
                let result = match &self.editor {
                    Some(editor) => dialogs::convert_palette(ctx, &mut state, editor, &palette),
                    None => dialogs::ConvertPaletteResult::Cancel,
                };
                match result {
                    dialogs::ConvertPaletteResult::Open => {
                        self.dialog = Some(Dialog::ConvertPalette(state));
                    }
                    dialogs::ConvertPaletteResult::Cancel => {}
                    dialogs::ConvertPaletteResult::Apply => {
                        self.execute_on_document(Command::ConvertToPalette(state.scope));
                    }
                }
            }
            Dialog::Tag(mut state) => match dialogs::frame_tag(ctx, &mut state) {
                dialogs::TagResult::Open => self.dialog = Some(Dialog::Tag(state)),
                dialogs::TagResult::Cancel => {}
                dialogs::TagResult::Apply => {
                    let command = match state.index {
                        Some(i) => Command::UpdateTag(i, state.tag),
                        None => Command::InsertTag(state.tag),
                    };
                    self.execute_on_document(command);
                }
                dialogs::TagResult::Delete => {
                    if let Some(i) = state.index {
                        self.execute_on_document(Command::DeleteTag(i));
                    }
                }
            },
            Dialog::Recovery(mut state) => match dialogs::recovery(ctx, &state) {
                dialogs::RecoveryChoice::Keep => self.dialog = Some(Dialog::Recovery(state)),
                dialogs::RecoveryChoice::Close => {}
                dialogs::RecoveryChoice::Delete(i) => {
                    let entry = state.entries.remove(i);
                    crate::services::recovery::delete(&entry);
                    if !state.entries.is_empty() {
                        self.dialog = Some(Dialog::Recovery(state));
                    }
                }
                dialogs::RecoveryChoice::OpenOriginal(i) => {
                    if self.editor.as_ref().is_some_and(Editor::is_modified) {
                        self.status_error("Save or close the current image first");
                        self.dialog = Some(Dialog::Recovery(state));
                    } else if let Some(path) = state.entries[i].original.clone() {
                        self.open_path(&path);
                        self.dialog = Some(Dialog::Recovery(state));
                    }
                }
                dialogs::RecoveryChoice::OpenRecovered(i) => {
                    if self.editor.as_ref().is_some_and(Editor::is_modified) {
                        self.status_error("Save or close the current image first");
                        self.dialog = Some(Dialog::Recovery(state));
                    } else {
                        let entry = state.entries[i].clone();
                        match entry.load() {
                            Ok(doc) => {
                                self.set_document(doc, None, format!("{} (recovered)", entry.name));
                                if let Some(editor) = &mut self.editor {
                                    editor.mark_unsaved();
                                }
                                self.recovery.adopt(entry);
                                self.success("Recovered — save it to keep it");
                                state.entries.remove(i);
                                if !state.entries.is_empty() {
                                    self.dialog = Some(Dialog::Recovery(state));
                                }
                            }
                            Err(e) => {
                                tracing::warn!("recovery copy {} unreadable: {e}", entry.id);
                                state.errors.insert(entry.id.clone(), e.to_string());
                                self.dialog = Some(Dialog::Recovery(state));
                            }
                        }
                    }
                }
            },
            Dialog::CanvasSize(mut state) => match dialogs::canvas_size(ctx, &mut state) {
                dialogs::CanvasSizeResult::Open => self.dialog = Some(Dialog::CanvasSize(state)),
                dialogs::CanvasSizeResult::Cancel => {}
                dialogs::CanvasSizeResult::Apply => self.execute_on_document(
                    Command::ResizeCanvas(state.width, state.height, state.anchor),
                ),
            },
            Dialog::About => match dialogs::about(ctx) {
                dialogs::AboutChoice::Keep => self.dialog = Some(Dialog::About),
                dialogs::AboutChoice::Licenses => self.dialog = Some(Dialog::Licenses(0)),
                dialogs::AboutChoice::Close => {}
            },
            Dialog::Licenses(mut tab) => {
                if dialogs::licenses(ctx, &mut tab) {
                    self.dialog = Some(Dialog::Licenses(tab));
                }
            }
            Dialog::ConfirmDeleteFrame(index) => {
                let message = format!(
                    "Frame {} has drawings on it. You can bring it back with Undo.",
                    index + 1
                );
                match dialogs::confirm(ctx, "Delete this frame?", &message, "Delete Frame") {
                    ConfirmChoice::Pending => self.dialog = Some(Dialog::ConfirmDeleteFrame(index)),
                    ConfirmChoice::Confirm => {
                        self.execute_on_document(Command::DeleteFrameConfirmed(index))
                    }
                    ConfirmChoice::Cancel => {}
                }
            }
        }
    }
}
