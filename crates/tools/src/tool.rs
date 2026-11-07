use tiny_skia::{Color, Mask, PixmapMut, Point};

/// Something drawn on the screenshot: a brush stroke, an arrow, a blurred
/// region...
pub trait Annotation {
    /// Draws onto `target`, a screenshot-sized canvas. `clip`, when given,
    /// limits drawing to the masked area.
    fn render(&self, target: &mut PixmapMut<'_>, clip: Option<&Mask>);
}

/// An annotation that is still being drawn (between press and release).
/// It renders itself as a live preview while in progress.
pub trait Gesture: Annotation {
    /// Pointer moved while the button is held.
    fn update(&mut self, to: Point);

    /// Pointer released. Returns the finished annotation, or `None` when the
    /// gesture produced nothing worth keeping.
    fn finish(self: Box<Self>) -> Option<Box<dyn Annotation>>;
}

/// A tool is a stateless factory of gestures plus a description of itself
/// for the UI. Being object-safe, tools are swapped at runtime as
/// `Box<dyn Tool>`.
pub trait Tool {
    fn meta(&self) -> ToolMeta;

    /// Pointer pressed at `at` with this tool active.
    fn begin(&self, at: Point, style: Style) -> Box<dyn Gesture>;
}

/// How a tool presents itself in the toolbar.
pub struct ToolMeta {
    pub name: &'static str,
    /// SVG path data on a 24x24 viewbox, drawn as a stroke.
    pub icon: &'static str,
    /// Which style settings the UI should offer for this tool.
    pub settings: &'static [Setting],
}

/// A user-adjustable parameter a tool reads from [`Style`].
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Setting {
    Color,
    /// Line width in logical pixels.
    Width { min: f32, max: f32 },
}

/// Current values of the settings, shared between tools.
#[derive(Debug, Clone, Copy)]
pub struct Style {
    pub color: Color,
    /// In screenshot pixels (the overlay already applied the output scale).
    pub width: f32,
}
