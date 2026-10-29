use super::{hint, section, status_line, toggle, SettingsAction, SettingsView};
use crate::config::Config;
use crate::integrations::discord_rpc::ConnectionState;
use crate::theme;

pub fn show(
    ui: &mut egui::Ui,
    config: &mut Config,
    view: &SettingsView,
    actions: &mut Vec<SettingsAction>,
) {
    section(ui, "Discord Rich Presence");
    hint(
        ui,
        "Shows in your Discord profile that you are working in Kagero. Uses the Discord \
         desktop app on this computer; no account details are needed.",
    );
    let d = &mut config.discord;
    let mut changed = toggle(ui, &mut d.enabled, "Enable Discord Rich Presence");

    section(ui, "Connection");
    // The connection runs in the background: keep its status fresh while visible.
    ui.ctx()
        .request_repaint_after(std::time::Duration::from_millis(500));
    if !view.discord_configured {
        status_line(ui, theme::TEXT_SECONDARY, "Not configured in this build");
    } else {
        match &view.discord_state {
            ConnectionState::NotConfigured => {
                status_line(ui, theme::TEXT_SECONDARY, "Not configured in this build")
            }
            ConnectionState::Disabled => status_line(ui, theme::TEXT_SECONDARY, "Off"),
            ConnectionState::Connecting => status_line(ui, theme::ACCENT, "Connecting…"),
            ConnectionState::Connected => status_line(ui, theme::SUCCESS, "Connected to Discord"),
            ConnectionState::NotConnected { reason, retry_in } => {
                let text = match retry_in {
                    Some(d) => format!("Not connected — {reason} (retrying in {}s)", d.as_secs()),
                    None => format!("Not connected — {reason}"),
                };
                status_line(ui, theme::TEXT_SECONDARY, &text);
            }
        }
        ui.horizontal(|ui| {
            if ui
                .add_enabled(d.enabled, egui::Button::new("Connect / Retry"))
                .clicked()
            {
                actions.push(SettingsAction::DiscordRetry);
            }
            if ui
                .add_enabled(
                    matches!(view.discord_state, ConnectionState::Connected),
                    egui::Button::new("Disconnect"),
                )
                .clicked()
            {
                actions.push(SettingsAction::DiscordDisconnect);
            }
        });
    }

    section(ui, "What to share");
    changed |= toggle(
        ui,
        &mut d.show_mode,
        "Show what I'm doing (drawing, animating…)",
    );
    changed |= toggle(ui, &mut d.show_project_name, "Show the project name");
    hint(
        ui,
        "Off by default. Only the file name is shared — never the folder, your user name or \
         anything from the image.",
    );
    changed |= toggle(ui, &mut d.show_timer, "Show elapsed time");
    changed |= toggle(ui, &mut d.show_button, "Show a “Get Kagero” button");

    section(ui, "Behavior");
    changed |= toggle(
        ui,
        &mut d.auto_reconnect,
        "Reconnect automatically when Discord starts",
    );
    changed |= toggle(
        ui,
        &mut d.clear_on_disable,
        "Clear my status when turned off",
    );
    if changed {
        actions.push(SettingsAction::Save);
    }
}
