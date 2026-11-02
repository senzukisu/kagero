//! Settings actions, themes, the updater and Discord status.

use super::*;

impl KageroApp {
    pub(super) fn execute_settings_action(&mut self, ctx: &egui::Context, action: SettingsAction) {
        match action {
            SettingsAction::Save => self.schedule_config_save(),
            SettingsAction::CheckUpdates => {
                self.updater.check(self.config.updates.include_prereleases)
            }
            SettingsAction::DownloadUpdate => self.updater.download(),
            SettingsAction::CancelDownload => self.updater.cancel(),
            SettingsAction::DismissUpdateError => self.updater.dismiss_error(),
            SettingsAction::InstallUpdate => {
                self.stop_playback();
                self.request(ctx, Pending::InstallUpdate);
            }
            SettingsAction::RestorePrevious => {
                self.stop_playback();
                self.request(ctx, Pending::RestorePrevious);
            }
            SettingsAction::DiscordRetry => self.discord.retry(),
            SettingsAction::PreviewTheme(name) => {
                if name == self.config.theme {
                    self.theme_preview = None;
                } else {
                    self.theme_preview = Some(name.clone());
                }
                self.show_theme(ctx, &name);
            }
            SettingsAction::KeepTheme => {
                if let Some(name) = self.theme_preview.take() {
                    self.config.theme = name;
                    self.schedule_config_save();
                }
            }
            SettingsAction::RevertTheme => {
                self.theme_preview = None;
                let saved = self.config.theme.clone();
                self.show_theme(ctx, &saved);
            }
            SettingsAction::ImportTheme => {
                let Some(path) = rfd::FileDialog::new()
                    .set_title("Import Theme")
                    .add_filter("Kagero theme", &["json"])
                    .pick_file()
                else {
                    return;
                };
                match self.themes.import(&path) {
                    Ok((theme, replaced)) => {
                        self.success(if replaced {
                            format!("Updated the theme \"{}\"", theme.name)
                        } else {
                            format!("Imported the theme \"{}\" — previewing it", theme.name)
                        });
                        self.execute_settings_action(ctx, SettingsAction::PreviewTheme(theme.name));
                    }
                    Err(e) => self.status_error(format!("Couldn't import the theme: {e}")),
                }
            }
            SettingsAction::ExportTheme(name) => {
                let Some(theme) = self.themes.find(&name) else {
                    return;
                };
                let Some(path) = rfd::FileDialog::new()
                    .set_title("Export Theme")
                    .add_filter("Kagero theme", &["json"])
                    .set_file_name(format!("{name}.json"))
                    .save_file()
                else {
                    return;
                };
                let path = crate::services::dialogs::with_extension(path, "json");
                let json = crate::themes::to_json(&theme);
                let written = crate::file_io::write_atomic(&path, |w| {
                    std::io::Write::write_all(w, json.as_bytes())?;
                    Ok(())
                });
                match written {
                    Ok(()) => self.success(format!("Exported the theme to {}", path.display())),
                    Err(e) => self.status_error(format!("Couldn't export the theme: {e}")),
                }
            }
            SettingsAction::DeleteTheme(name) => match self.themes.delete(&name) {
                Ok(()) => {
                    if self.config.theme == name {
                        self.config.theme = crate::themes::DEFAULT_THEME.to_owned();
                        self.schedule_config_save();
                    }
                    if self.theme_preview.as_deref() == Some(name.as_str())
                        || self.theme_preview.is_none()
                    {
                        self.theme_preview = None;
                        let saved = self.config.theme.clone();
                        self.show_theme(ctx, &saved);
                    }
                    self.info(format!("Deleted the theme \"{name}\""));
                }
                Err(e) => self.status_error(format!("Couldn't delete the theme: {e}")),
            },
            SettingsAction::OpenThemesFolder => {
                if let Some(dir) = self.themes.dir() {
                    let _ = std::fs::create_dir_all(dir);
                    crate::ui::export_dialog::open_folder(dir);
                }
            }
            SettingsAction::ReloadThemes => {
                self.themes.reload();
                let shown = self
                    .theme_preview
                    .clone()
                    .unwrap_or_else(|| self.config.theme.clone());
                self.show_theme(ctx, &shown);
            }
            SettingsAction::OpenPluginsFolder => {
                if let Some(dir) = self.plugins.dir() {
                    let _ = std::fs::create_dir_all(dir);
                    crate::ui::export_dialog::open_folder(dir);
                }
            }
            SettingsAction::ReloadPlugins => self.plugins.reload(),
            SettingsAction::InstallExamplePlugin => match self.plugins.install_example() {
                Ok(_) => self.success("Installed \"Example Tools\" — turn it on below"),
                Err(e) => self.status_error(format!("Couldn't install the example: {e}")),
            },
            SettingsAction::OpenRecoveryFolder => {
                crate::ui::export_dialog::open_folder(self.recovery.dir());
            }
            SettingsAction::DiscordDisconnect => self
                .discord
                .disconnect(self.config.discord.clear_on_disable),
        }
    }

    pub(super) fn poll_updates(&mut self) {
        for event in self.updater.take_events() {
            match event {
                UpdateEvent::CheckFinished { found } => {
                    self.config.updates.last_check_unix = Some(updater::unix_now());
                    self.schedule_config_save();
                    if let Some(version) = found {
                        let new = self.notified_version.as_ref() != Some(&version);
                        if new && self.config.updates.notify && !self.settings.open {
                            self.info(format!(
                                "Kagero {version} is available — Help › Check for Updates"
                            ));
                        }
                        self.notified_version = Some(version);
                    }
                }
                UpdateEvent::CheckFailed(_) => {}
                UpdateEvent::DownloadFinished => {
                    if !self.settings.open {
                        self.success("Update downloaded — see Settings › Updates");
                    }
                }
            }
        }
        // Automatic checks: once at startup, then by interval, never while busy.
        let now = Instant::now();
        let first = self.last_auto_check_poll.is_none();
        if !first
            && self
                .last_auto_check_poll
                .is_some_and(|t| now.duration_since(t) < AUTO_CHECK_POLL)
        {
            return;
        }
        self.last_auto_check_poll = Some(now);
        let u = &self.config.updates;
        let due = (first && u.check_on_startup)
            || (u.periodic_check
                && updater::check_due(u.last_check_unix, u.interval_hours, updater::unix_now()));
        if due
            && !self.updater.is_busy()
            && matches!(
                self.updater.status(),
                UpdateStatus::Idle | UpdateStatus::UpToDate | UpdateStatus::Failed(_)
            )
        {
            self.updater.check(u.include_prereleases);
        }
    }

    pub(super) fn update_presence(&mut self) {
        let mode = match &self.editor {
            None => AppMode::Home,
            Some(_) if self.export.open => AppMode::Exporting,
            Some(_) if self.player.is_playing() => AppMode::Previewing,
            Some(e) if e.document().frame_count() > 1 => AppMode::AnimationDocument,
            Some(_) => AppMode::StaticDocument,
        };
        let name = self.editor.as_ref().map(|e| e.name().to_owned());
        let desired = build_presence(
            mode,
            name.as_deref(),
            self.session_start_unix,
            &self.config.discord,
        );
        self.discord
            .update(desired, Flags::from(&self.config.discord));
    }

    /// Shows a theme on the whole window (the default theme if it no longer exists).
    pub(super) fn show_theme(&mut self, ctx: &egui::Context, name: &str) {
        let colors = self
            .themes
            .find(name)
            .map_or(crate::theme::EMBER, |t| t.colors);
        crate::theme::set_colors(colors);
        theme::apply(ctx);
    }
}
