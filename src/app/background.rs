//! Work that runs alongside editing: plugin commands and autosave.

use super::*;

impl KageroApp {
    /// Starts a plugin command on the active layer in the background.
    pub(super) fn run_plugin(&mut self, plugin_id: &str, command: &str) {
        if self.plugin_job.is_some() {
            self.info("A plugin is still running — wait or press Esc to stop it");
            return;
        }
        let Some(editor) = &mut self.editor else {
            return;
        };
        self.player.stop(editor);
        editor.end_stroke();
        let enabled = self.config.plugins.enabled.iter().any(|e| e == plugin_id);
        let Some(plugin) = self.plugins.find(plugin_id).filter(|_| enabled) else {
            self.status_error("That plugin is turned off or no longer installed");
            return;
        };
        let doc = editor.document();
        let input = crate::plugins::PluginInput {
            width: doc.width(),
            height: doc.height(),
            pixels: doc.active_cel().as_bytes().to_vec(),
            selection: editor.selection_shape().cloned(),
            primary: editor.primary,
            secondary: editor.secondary,
        };
        let target = (doc.active_frame().id, doc.active_layer().id);
        match crate::plugins::PluginJob::spawn(plugin, command, input, editor.history().state_id())
        {
            Ok(job) => {
                self.info(format!("Running {}… (Esc stops it)", job.title));
                self.plugin_job = Some((job, target.0, target.1));
            }
            Err(e) => self.status_error(format!("Couldn't start the plugin: {e}")),
        }
    }

    /// Applies a finished plugin command (one undo step), if the image didn't change.
    pub(super) fn poll_plugin(&mut self, ctx: &egui::Context) {
        let Some((job, frame, layer)) = &self.plugin_job else {
            return;
        };
        let Some(result) = job.poll() else {
            ctx.request_repaint_after(std::time::Duration::from_millis(50));
            return;
        };
        let (job, frame, layer) = (job.title.clone(), *frame, *layer);
        let state_id = self.plugin_job.as_ref().map(|j| j.0.state_id);
        self.plugin_job = None;
        let output = match result {
            Ok(o) => o,
            Err(e) => {
                self.status_error(format!("{job}: {e}"));
                return;
            }
        };
        let Some(editor) = &mut self.editor else {
            return;
        };
        let doc = editor.document();
        let same = Some(editor.history().state_id()) == state_id
            && doc.active_frame().id == frame
            && doc.active_layer().id == layer;
        if !same {
            self.status_error(format!(
                "{job}: the image changed while it ran, so nothing was applied"
            ));
            return;
        }
        if let Some(c) = output.primary {
            editor.primary = c;
        }
        let applied = match output.pixels {
            Some(px) => editor.replace_active_pixels("Plugin", px),
            None => Ok(false),
        };
        let note = output.messages.join(" · ");
        match applied {
            Ok(true) if note.is_empty() => self.success(format!("{job}: done")),
            Ok(_) if !note.is_empty() => self.success(format!("{job}: {note}")),
            Ok(_) => self.info(format!("{job}: nothing changed")),
            Err(e) => self.status_error(format!("{job}: {e}")),
        }
    }

    /// Writes a recovery copy of a document with unsaved changes every few minutes.
    pub(super) fn autosave(&mut self, ctx: &egui::Context) {
        if let Some(crate::services::recovery::WriteResult::Failed(e)) = self.recovery.poll() {
            self.status_error(format!("Autosave failed: {e}"));
        }
        let settings = self.config.autosave;
        let Some(editor) = &self.editor else {
            return;
        };
        if !settings.enabled || !editor.is_modified() || editor.is_stroking() {
            return;
        }
        let state = editor.history().state_id();
        if !self.recovery.needs_copy(state) || self.recovery.is_writing() {
            return;
        }
        let interval = Duration::from_secs(u64::from(settings.interval_minutes) * 60);
        let elapsed = self.last_autosave.elapsed();
        if elapsed < interval {
            // Come back when it's due, even if nothing else happens.
            ctx.request_repaint_after(interval - elapsed);
            return;
        }
        let doc = editor.document().clone();
        let name = editor.name().to_owned();
        let original = editor.path().map(Path::to_path_buf);
        if self.recovery.write(doc, state, &name, original.as_deref()) {
            self.last_autosave = Instant::now();
            ctx.request_repaint_after(Duration::from_millis(250));
        }
    }
}
