use crate::ui::App;
use eframe::egui;
use ksni::{ToolTip, Tray};
use tokio::sync::broadcast::Sender;


pub struct AppTray {
    pub tx: Sender<bool>,
    pub can_open_gui: bool,
}

impl Tray for AppTray {
    fn id(&self) -> String {
        env!("CARGO_PKG_NAME").into()
    }
    fn activate(&mut self, _x: i32, _y: i32) {
        if !self.can_open_gui {
            return
        }

        self.tx.send(true).unwrap();
    }
    fn title(&self) -> String {
        "BW SSH Agent".into()
    }

    fn icon_name(&self) -> String {
        "help-about".into()
    }

    fn tool_tip(&self) -> ToolTip {
        ToolTip {
            title: "BW SSH Agent".into(),
            ..Default::default()
        }
    }

    fn menu(&self) -> Vec<ksni::MenuItem<Self>> {
        use ksni::menu::*;
        vec![
            StandardItem {
                label: "Open".into(),
                enabled: self.can_open_gui,
                activate: Box::new(|s: &mut AppTray| {
                    println!("activate");
                    s.tx.send(true).unwrap();
                }),
                ..Default::default()
            }
            .into(),
            MenuItem::Separator,
            StandardItem {
                label: "Exit".into(),
                icon_name: "application-exit".into(),
                activate: Box::new(|_| std::process::exit(0)),
                ..Default::default()
            }
            .into(),
        ]
    }
}
