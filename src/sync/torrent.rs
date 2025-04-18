// src/sync/torrent.rs

// This module will handle interactions with librqbit for the sync process.
// - Adding the initial/updated torrent
// - Forgetting the old torrent
// - Monitoring torrent status for sync purposes (e.g., completion)

use crate::config::AppConfig;
use crate::ui::utils::SyncStatus;
use crate::sync::messages::SyncEvent;
use anyhow::{Context, Result};
use librqbit::{AddTorrent, AddTorrentOptions};
use tokio::sync::mpsc;
use librqbit::limits::LimitsConfig;
use std::num::NonZeroU32;

use super::utils::send_sync_status_event;

// Function to manage the torrent task based on config
pub async fn manage_torrent_task(
    app_config: &AppConfig,
    api: &librqbit::api::Api,
    ui_tx: &mpsc::UnboundedSender<SyncEvent>,
    current_id_to_forget: Option<usize>,
    torrent_content: Vec<u8>,
) -> Result<Option<usize>> {
    println!(
        "Sync: Managing torrent task for URL: {}. Path: {}. Current ID to forget: {:?}",
        app_config.torrent_url,
        app_config.download_path.display(),
        current_id_to_forget
    );

    // 1. Forget the old torrent if an ID was provided
    if let Some(id_to_forget) = current_id_to_forget {
        println!("Sync: Forgetting previous torrent ID: {}", id_to_forget);
        send_sync_status_event(ui_tx, SyncStatus::UpdatingTorrent);
        
        match api
            .api_torrent_action_forget(id_to_forget.into())
            .await
        {
            Ok(_) => println!("Sync: Successfully forgot torrent {}", id_to_forget),
            Err(e) => {
                // Log error but proceed, maybe the torrent was already gone
                eprintln!(
                    "Sync: Error forgetting torrent {}: {}. Proceeding to add new one.",
                    id_to_forget,
                    e
                );
                 let _ = ui_tx.send(SyncEvent::Error(format!("Error forgetting old torrent {}: {}", id_to_forget, e)));
            }
        }
    }

    // 2. Add the new torrent
    println!(
        "Sync: Adding new torrent content ({} bytes) to path: {}",
        torrent_content.len(),
        app_config.download_path.display()
    );

    if app_config.download_path.as_os_str().is_empty() {
        println!("Sync: Download path is empty, cannot add torrent.");
        let err_msg = "Download path not configured".to_string();
        let _ = ui_tx.send(SyncEvent::Error(err_msg.clone()));
        send_sync_status_event(ui_tx, SyncStatus::Error(err_msg));
        // Return Ok(None) as no torrent was added
        return Ok(None);
    }

    // Notify that we're still updating - librqbit will do the checking internally
    send_sync_status_event(ui_tx, SyncStatus::UpdatingTorrent);

    let add_request = AddTorrent::from_bytes(torrent_content);
    
    // Create a LimitsConfig based on app settings
    let ratelimits = LimitsConfig {
        // Convert KB/s to B/s (bytes per second) and to NonZeroU32
        download_bps: app_config.max_download_speed.and_then(|s| {
            let value = (s * 1024) as u32;
            NonZeroU32::new(value)
        }),
        upload_bps: app_config.max_upload_speed.and_then(|s| {
            let value = (s * 1024) as u32;
            NonZeroU32::new(value)
        }),
    };
    
    let options = AddTorrentOptions {
        output_folder: Some(app_config.download_path.to_string_lossy().into_owned()),
        overwrite: true, // Important: ensures librqbit checks existing files
        paused: !app_config.should_seed, // Opposite of should_seed
        ratelimits,
        ..Default::default()
    };

    println!(
        "Sync: Applying settings - Seeding: {}, Upload limit: {:?} KB/s, Download limit: {:?} KB/s",
        app_config.should_seed,
        app_config.max_upload_speed,
        app_config.max_download_speed
    );

    let response = api
        .api_add_torrent(add_request, Some(options))
        .await
        .context("Failed to add torrent via librqbit API")?;

    if let Some(id) = response.id {
        println!("Sync: Torrent added successfully with ID: {}", id);
        let _ = ui_tx.send(SyncEvent::TorrentAdded(id));
        
        // Return to Idle after adding - state tracking is now separate from torrent state
        send_sync_status_event(ui_tx, SyncStatus::Idle);
        
        Ok(Some(id))
    } else {
        println!("Sync: Torrent added but no ID returned by API.");
        // Maybe send an error/warning? For now, return Ok(None)
        let err_msg = "Torrent added but API returned no ID".to_string();
        let _ = ui_tx.send(SyncEvent::Error(err_msg.clone()));
        send_sync_status_event(ui_tx, SyncStatus::Error(err_msg));
        Ok(None)
    }
}