impl BaseScreen {
    /// Create a new BaseScreen
    pub fn new(ctx: Rc<egui::Context>) -> Self {
        Self {
            ctx,
        }
    }
}