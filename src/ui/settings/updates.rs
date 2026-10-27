use egui::RichText;

use super::{hint, section, status_line, toggle, SettingsAction, SettingsView};
use crate::config::{Config, UPDATE_INTERVALS_HOURS};
use crate::release;
use crate::services::updater::{ReleaseInfo, UpdateStatus};
use crate::theme;
use crate::ui::dialogs::accent_button;

fn megabytes(bytes: u64) -> String {
    format!("{:.1} MB", bytes as f64 / (1024.0 * 1024.0))
}

pub fn show(
    ui: &mut egui::Ui,
    config: &mut Config,
    view: &SettingsView,
    actions: &mut Vec<SettingsAction>,
) {
    section(ui, "Kagero");
    ui.label(
        RichText::new(format!(
            "Version {} {}",
            crate::APP_VERSION,
            crate::APP_CHANNEL
        ))
        .strong()
        .color(theme::TEXT_PRIMARY),
    );
    let build = if release::is_official_build() {
        "Official release build"
    } else {
        "Development build — updates can be checked and downloaded, but not installed"
    };
    hint(ui, build);
    ui.hyperlink_to("Releases page", release::releases_url());

    section(ui, "Status");
    match &view.update_status {
        UpdateStatus::Idle => status_line(ui, theme::TEXT_SECONDARY, "Not checked yet"),
        UpdateStatus::Checking => {
            ui.horizontal(|ui| {
                ui.spinner();
                ui.label("Checking GitHub for updates…");
            });
        }
        UpdateStatus::UpToDate => status_line(ui, theme::SUCCESS, "Kagero is up to date"),
        UpdateStatus::Available(info) => {
            status_line(
                ui,
                theme::ACCENT,
                &format!("Version {} is available", info.version),
            );
            release_card(ui, info);
            if info.package.is_some() {
                if ui.add(accent_button("Download Update")).clicked() {
                    actions.push(SettingsAction::DownloadUpdate);
                }
            } else {
                hint(ui, "This release has no package for your system. Download it from the release page.");
            }
        }
        UpdateStatus::Downloading {
            release,
            received,
            total,
        } => {
            status_line(
                ui,
                theme::ACCENT,
                &format!("Downloading {}…", release.version),
            );
            let fraction = total.map_or(0.0, |t| *received as f32 / t.max(1) as f32);
            ui.add(
                egui::ProgressBar::new(fraction)
                    .desired_width(380.0)
                    .text(match total {
                        Some(t) => format!("{} of {}", megabytes(*received), megabytes(*t)),
                        None => megabytes(*received),
                    }),
            );
            if ui.button("Cancel").clicked() {
                actions.push(SettingsAction::CancelDownload);
            }
        }
        UpdateStatus::Downloaded { release, package } => {
            status_line(
                ui,
                theme::SUCCESS,
                &format!("Version {} downloaded", release.version),
            );
            ui.label(
                RichText::new(format!("SHA-256 {}", package.sha256))
                    .small()
                    .monospace()
                    .color(theme::TEXT_SECONDARY),
            );
            if package.signature_verified {
                status_line(
                    ui,
                    theme::SUCCESS,
                    "Signature verified (official Onibi Lab release)",
                );
            } else {
                status_line(
                    ui,
                    theme::ERROR,
                    "Authenticity not verified — this build has no update signing key, so it won't install",
                );
            }
            let installable = package.signature_verified && view.can_install;
            if ui
                .add_enabled(installable, accent_button("Install and Restart"))
                .on_disabled_hover_text(
                    "Installing needs an official Windows build and a verified signature. \
                     You can still install manually from the releases page.",
                )
                .clicked()
            {
                actions.push(SettingsAction::InstallUpdate);
            }
            hint(ui, &format!("File: {}", package.path.display()));
            release_card(ui, release);
        }
        UpdateStatus::Failed(message) => {
            status_line(ui, theme::ERROR, "The last update action failed");
            ui.label(RichText::new(message).color(theme::TEXT_SECONDARY));
            if ui.small_button("Dismiss").clicked() {
                actions.push(SettingsAction::DismissUpdateError);
            }
        }
    }
    ui.add_space(4.0);
    let busy = matches!(
        view.update_status,
        UpdateStatus::Checking | UpdateStatus::Downloading { .. }
    );
    if ui
        .add_enabled(!busy, egui::Button::new("Check for Updates"))
        .clicked()
    {
        actions.push(SettingsAction::CheckUpdates);
    }

    section(ui, "Automatic checks");
    let u = &mut config.updates;
    let mut changed = false;
    changed |= toggle(ui, &mut u.check_on_startup, "Check when Kagero starts");
    changed |= toggle(
        ui,
        &mut u.periodic_check,
        "Check periodically while running",
    );
    ui.horizontal(|ui| {
        ui.label("Every");
        egui::ComboBox::from_id_salt("update-interval")
            .selected_text(format!("{} hours", u.interval_hours))
            .show_ui(ui, |ui| {
                for h in UPDATE_INTERVALS_HOURS {
                    changed |= ui
                        .selectable_value(&mut u.interval_hours, h, format!("{h} hours"))
                        .changed();
                }
            });
    });
    changed |= toggle(
        ui,
        &mut u.include_prereleases,
        "Include prereleases (betas)",
    );
    changed |= toggle(ui, &mut u.notify, "Tell me when an update is found");
    if changed {
        actions.push(SettingsAction::Save);
    }

    if view.has_backup {
        section(ui, "Recovery");
        hint(
            ui,
            "The version installed before the last update is kept as a backup.",
        );
        if ui
            .add_enabled(
                view.can_install,
                egui::Button::new("Restore Previous Version"),
            )
            .clicked()
        {
            actions.push(SettingsAction::RestorePrevious);
        }
    }
}

fn release_card(ui: &mut egui::Ui, info: &ReleaseInfo) {
    egui::Frame::new()
        .fill(theme::SURFACE)
        .corner_radius(6)
        .inner_margin(egui::Margin::same(10))
        .show(ui, |ui| {
            ui.set_width(400.0);
            ui.horizontal(|ui| {
                ui.label(RichText::new(&info.title).strong());
                if info.prerelease {
                    ui.label(RichText::new("prerelease").small().color(theme::ACCENT));
                }
            });
            let mut meta = Vec::new();
            if !info.published.is_empty() {
                meta.push(format!("Released {}", info.published));
            }
            if let Some(p) = &info.package {
                meta.push(format!("Download {}", megabytes(p.size)));
            }
            if !meta.is_empty() {
                hint(ui, &meta.join("  ·  "));
            }
            if !info.notes.trim().is_empty() {
                egui::ScrollArea::vertical()
                    .id_salt("release-notes")
                    .max_height(110.0)
                    .show(ui, |ui| {
                        ui.label(RichText::new(&info.notes).small());
                    });
            }
            ui.hyperlink_to("Open release page", &info.page_url);
        });
}
