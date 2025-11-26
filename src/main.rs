use eframe::{App, egui, run_native};

mod api_service;
mod protocol;
mod usb_service;

struct  BindKeyApp {
    name:  String,
}

impl BindKeyApp {
    // _cc sert à configurer le style au démarrage si besoin (on l'ignore pour l'instant)
    fn new(_cc: &eframe::CreationContext<'_>) -> Self {
        Self {
            name: String::from("BindKey Alpha"),
        }
    }
}

impl eframe::App for BindKeyApp {
    // Cette fonction est appelée 60 fois par seconde
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        
        // C'est ici qu'on dessine.
        // Utilise 'egui::CentralPanel' pour créer le panneau principal.
        egui::CentralPanel::default().show(ctx, |ui| {
            
            // EXERCICE :
            // Essaye d'afficher un titre ici avec ui.heading("...");
            ui.heading("Yo les bg");
            
            // Essaye d'afficher un texte simple avec ui.label("...");
            ui.label("Salut les mecs");
            
            if ui.button("Cliquez ici").clicked() {
                // On peut mettre de la logique ici plus tard
                println!("Bouton cliqué !");
            }
        });
    }
}

fn main() -> eframe::Result  {
    let native_options = eframe::NativeOptions::default();
    run_native("BindKey Client", native_options, Box::new(|cc| Ok(Box::new(BindKeyApp::new(cc)))))
}
