/// A trait for progress bars
pub trait ProgressBar {
    /// Set the progress
    fn set_progress(&mut self, progress: f32);

    /// Set the text
    fn set_text(&mut self, text: &str);
}