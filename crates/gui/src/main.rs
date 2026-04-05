mod tray;
mod ui;

use crate::tray::AppTray;
use crate::ui::App;
use anyhow::Error;
use eframe::egui;
use ksni::TrayMethods;
use log::LevelFilter;
use simple_logger::SimpleLogger;
use tokio::sync::broadcast;

#[global_allocator]
static GLOBAL: std::alloc::System = std::alloc::System;

#[tokio::main]
async fn main() -> Result<(), Error> {
    SimpleLogger::new()
        .with_level(LevelFilter::Info)
        .with_module_level("zbus", LevelFilter::Warn)
        .init()?;

    let (tx, mut rx) = broadcast::channel::<bool>(1);

    let mut tray_rx = rx.resubscribe();
    let tray_tx = tx.clone();

    tokio::spawn(async move {
        let app_tray = AppTray {
            tx: tray_tx,
            can_open_gui: true,
        };

        let handle = app_tray.spawn().await.unwrap();

        while let Ok(show) = tray_rx.recv().await {
            handle.update(|s| s.can_open_gui = !show).await;
        }
    });

    tokio::spawn(async move {
        loop {
            release_memory_to_os();
            tokio::time::sleep(std::time::Duration::from_secs(10)).await;
        }
    });

    let main_thread_future = async {
        while let Ok(show) = rx.recv().await {
            if show {
                let app = App {};
                let options = eframe::NativeOptions {
                    viewport: egui::ViewportBuilder::default().with_inner_size([320.0, 240.0]),
                    persist_window: false,
                    centered: true,
                    ..Default::default()
                };
                eframe::run_native("BW SSH Agent", options, Box::new(|_| Ok(Box::new(app))))
                    .expect("Failed to show gui");

                tx.send(false).expect("failed to send show message");

            }
        }
    };

    tokio::select! {
        _ = main_thread_future => {}
        _ = tokio::signal::ctrl_c() => {}
    }

    Ok(())
}

#[cfg(target_os = "linux")]
fn release_memory_to_os() {
    unsafe {
        libc::malloc_trim(0); // releases free heap pages back to OS
    }
}
