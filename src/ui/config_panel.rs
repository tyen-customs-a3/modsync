// src/ui/config_panel.rs
// Component for configuration UI

use crate::app::MyApp;
use crate::actions::save_config_changes;
use crate::ui::UiMessage;
use eframe::egui::{self, RichText};

/// Component for handling configuration settings
pub struct ConfigPanel;

impl ConfigPanel {
    /// Draw the configuration panel
    pub fn draw(ui: &mut egui::Ui, app: &mut MyApp) {
        ui.heading("ModSync Configuration");
        ui.separator();

        // URL input
        ui.horizontal(|ui| {
            ui.label("Remote Torrent URL:");
            ui.text_edit_singleline(&mut app.config_edit_url);
        });
        
        // Path input
        ui.horizontal(|ui| {
            ui.label("Local Download Path:");
            ui.text_edit_singleline(&mut app.config_edit_path_str);
        });

        // Buttons row
        ui.horizontal(|ui| {
            // Save config button
            if ui.button("Save Configuration").clicked() {
                save_config_changes(app);
            }
            
            // Refresh button (only enabled when config is valid)
            Self::draw_refresh_button(ui, app);

            // Verify button (only enabled when config is valid)
            Self::draw_verify_button(ui, app);
        });

        ui.separator();
        
        // Sync status display
        Self::draw_sync_status(ui, app);
        
        ui.separator();
    }
    
    /// Draw the refresh button
    fn draw_refresh_button(ui: &mut egui::Ui, app: &mut MyApp) {
        // Enable button only when config is valid
        let is_config_valid = !app.config.torrent_url.is_empty() && 
                             !app.config.download_path.as_os_str().is_empty();
        
        if ui.add_enabled(
            is_config_valid,
            egui::Button::new("Check for Updates")
        ).clicked() {
            println!("UI: Manual refresh requested");
            if let Err(e) = app.sync_cmd_tx.send(UiMessage::TriggerManualRefresh) {
                eprintln!("UI: Failed to send manual refresh request: {}", e);
            }
        }
    }
    
    /// Draw the verify local files button
    fn draw_verify_button(ui: &mut egui::Ui, app: &mut MyApp) {
        // Enable button only when config is valid
        let is_config_valid = !app.config.torrent_url.is_empty() && 
                             !app.config.download_path.as_os_str().is_empty();

        if ui.add_enabled(
            is_config_valid,
            egui::Button::new("Verify Local Files")
        ).clicked() {
            println!("UI: Verify local files requested");
            if let Err(e) = app.sync_cmd_tx.send(UiMessage::TriggerFolderVerify) {
                eprintln!("UI: Failed to send folder verify request: {}", e);
            }
        }
    }
    
    /// Draw the sync status display
    fn draw_sync_status(ui: &mut egui::Ui, app: &MyApp) {
        ui.horizontal(|ui| {
            ui.label("Sync Status: ");
            ui.label(
                RichText::new(app.sync_status.display_text())
                    .color(app.sync_status.display_color())
                    .strong()
            );
        });
    }
} 