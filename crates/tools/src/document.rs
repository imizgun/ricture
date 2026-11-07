use crate::tool::{Annotation, Gesture, Style, Tool};
use tiny_skia::{Mask, PixmapMut, Point};

/// The annotations placed on the screenshot, plus the one being drawn.
#[derive(Default)]
pub struct Document {
    items: Vec<Box<dyn Annotation>>,
    active: Option<Box<dyn Gesture>>,
    /// Bumped whenever `items` changes, so renderers can cache the
    /// flattened committed annotations and rebuild only when needed.
    revision: u64,
}

impl Document {
    pub fn begin(&mut self, tool: &dyn Tool, at: Point, style: Style) {
        self.active = Some(tool.begin(at, style));
    }

    pub fn update(&mut self, to: Point) {
        if let Some(gesture) = &mut self.active {
            gesture.update(to);
        }
    }

    pub fn finish(&mut self) {
        if let Some(annotation) = self.active.take().and_then(Gesture::finish) {
            self.items.push(annotation);
            self.revision += 1;
        }
    }

    pub fn is_drawing(&self) -> bool {
        self.active.is_some()
    }

    /// Removes the most recent annotation. Returns whether anything changed.
    pub fn undo(&mut self) -> bool {
        let removed = self.items.pop().is_some();
        if removed {
            self.revision += 1;
        }
        removed
    }

    pub fn revision(&self) -> u64 {
        self.revision
    }

    /// Draws every finished annotation, oldest first.
    pub fn render_committed(&self, target: &mut PixmapMut<'_>) {
        for item in &self.items {
            item.render(target, None);
        }
    }

    /// Draws the in-progress gesture, if any.
    pub fn render_active(&self, target: &mut PixmapMut<'_>, clip: Option<&Mask>) {
        if let Some(gesture) = &self.active {
            gesture.render(target, clip);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Brush;
    use tiny_skia::{Color, Pixmap};

    fn red(width: f32) -> Style {
        Style { color: Color::from_rgba8(255, 0, 0, 255), width }
    }

    fn pixel(pixmap: &Pixmap, x: u32, y: u32) -> [u8; 4] {
        let c = pixmap.pixel(x, y).unwrap();
        [c.red(), c.green(), c.blue(), c.alpha()]
    }

    #[test]
    fn brush_stroke_is_committed_and_undone() {
        let mut doc = Document::default();
        doc.begin(&Brush, Point::from_xy(10.0, 50.0), red(6.0));
        for x in (12..=90).step_by(2) {
            doc.update(Point::from_xy(x as f32, 50.0));
        }
        assert!(doc.is_drawing());
        doc.finish();
        assert!(!doc.is_drawing());
        assert_eq!(doc.revision(), 1);

        let mut canvas = Pixmap::new(100, 100).unwrap();
        doc.render_committed(&mut canvas.as_mut());
        assert_eq!(pixel(&canvas, 50, 50), [255, 0, 0, 255], "on the stroke");
        assert_eq!(pixel(&canvas, 50, 20), [0, 0, 0, 0], "away from it");

        assert!(doc.undo());
        assert_eq!(doc.revision(), 2);
        let mut canvas = Pixmap::new(100, 100).unwrap();
        doc.render_committed(&mut canvas.as_mut());
        assert_eq!(pixel(&canvas, 50, 50), [0, 0, 0, 0]);
    }

    #[test]
    fn click_leaves_a_dot_and_clip_is_respected() {
        let mut doc = Document::default();
        doc.begin(&Brush, Point::from_xy(50.0, 50.0), red(10.0));

        let mut clip = tiny_skia::Mask::new(100, 100).unwrap();
        let left_half = tiny_skia::PathBuilder::from_rect(tiny_skia::Rect::from_xywh(0.0, 0.0, 50.0, 100.0).unwrap());
        clip.fill_path(&left_half, tiny_skia::FillRule::Winding, false, tiny_skia::Transform::identity());

        let mut canvas = Pixmap::new(100, 100).unwrap();
        doc.render_active(&mut canvas.as_mut(), Some(&clip));
        assert_eq!(pixel(&canvas, 47, 50), [255, 0, 0, 255], "inside clip");
        assert_eq!(pixel(&canvas, 53, 50), [0, 0, 0, 0], "outside clip");
    }
}
