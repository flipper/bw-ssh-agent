use eframe::egui;
use std::default::Default;

pub struct App {
}

impl eframe::App for App {
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        egui::CentralPanel::default().show_inside(ui, |ui| {
            ui.heading("My egui Application");
            ui.button("Login")
        });
    }
}
