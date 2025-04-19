impl UncivApp {
    /// Create a new UncivApp
    pub fn new() -> Self {
        // Create the base screen
        let base_screen = Rc::new(BaseScreen::new(Rc::new(eframe::egui::Context::default())));

        Self {
            base_screen,
            current_screen: None,
        }
    }
}