/// Common font metrics abstraction used across different font implementations
///
/// Implementations of FontImplementation will use different FontMetrics - AWT or Android.Paint,
/// both have a class of that name, no other common point: thus we create an abstraction.
///
/// This is used by `Fonts.get_pixmap_from_actor` for vertical positioning.
#[derive(Debug, Clone, Copy)]
pub struct FontMetricsCommon {
    /// (positive) distance from the baseline up to the recommended top of normal text
    pub ascent: f32,

    /// (positive) distance from the baseline down to the recommended bottom of normal text
    pub descent: f32,

    /// (positive) maximum distance from top to bottom of any text,
    /// including potentially empty space above ascent or below descent
    pub height: f32,

    /// Space from the bounding box top to the top of the ascenders - includes line spacing and
    /// room for unusually high ascenders, as `ascent` is only a recommendation.
    ///
    /// Note: This is NOT what typographical leading actually is, but redefined as extra empty space
    /// on top, to make it easier to sync desktop and android. AWT has some leading but no measures
    /// outside ascent+descent+leading, while Android has its leading always 0 but typically top
    /// above ascent and bottom below descent.
    /// I chose to map AWT's spacing to the top as I found the calculations easier to visualize.
    pub leading: f32,
}

impl FontMetricsCommon {
    /// Creates a new FontMetricsCommon instance
    pub fn new(ascent: f32, descent: f32, height: f32, leading: f32) -> Self {
        Self {
            ascent,
            descent,
            height,
            leading,
        }
    }
}