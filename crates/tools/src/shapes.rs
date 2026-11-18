use crate::tool::{Annotation, Gesture, Setting, Style, Tool, ToolMeta};
use tiny_skia::{LineCap, LineJoin, Paint, PathBuilder, PixmapMut, Point, Stroke, Transform};

/// Both tools here are drawn with a stroke, so they share the brush's
/// settings.
const SETTINGS: &[Setting] = &[Setting::Color, Setting::Width { min: 1.0, max: 32.0 }];

/// A straight line from the press to the release.
pub struct Line;

impl Tool for Line {
    fn meta(&self) -> ToolMeta {
        ToolMeta {
            id: "line",
            name: "Line",
            icon: "M5 19L19 5",
            settings: SETTINGS,
        }
    }

    fn begin(&self, at: Point, style: Style) -> Box<dyn Gesture> {
        Box::new(Segment { from: at, to: at, style, head: false })
    }
}

/// A line with an arrowhead where the pointer was released.
pub struct Arrow;

impl Tool for Arrow {
    fn meta(&self) -> ToolMeta {
        ToolMeta {
            id: "arrow",
            name: "Arrow",
            icon: "M5 19L19 5M9 5H19V15",
            settings: SETTINGS,
        }
    }

    fn begin(&self, at: Point, style: Style) -> Box<dyn Gesture> {
        Box::new(Segment { from: at, to: at, style, head: true })
    }
}

/// A line between two points, optionally with an arrowhead at `to`.
struct Segment {
    from: Point,
    to: Point,
    style: Style,
    head: bool,
}

/// Drags shorter than this (in px) are a stray click, not a shape.
const MIN_LENGTH: f32 = 2.0;
/// Arrowhead length relative to the stroke width, and its floor in px, so a
/// thin arrow still ends in a visible head.
const HEAD_SCALE: f32 = 3.5;
const HEAD_MIN: f32 = 9.0;
/// How far each barb opens from the shaft.
const HEAD_ANGLE_DEG: f32 = 28.0;

impl Gesture for Segment {
    fn update(&mut self, to: Point) {
        self.to = to;
    }

    fn finish(self: Box<Self>) -> Option<Box<dyn Annotation>> {
        let length = self.from.distance(self.to);
        (length >= MIN_LENGTH).then_some(self as Box<dyn Annotation>)
    }
}

impl Annotation for Segment {
    fn render(&self, target: &mut PixmapMut<'_>) {
        let length = self.from.distance(self.to);
        if length < MIN_LENGTH {
            return;
        }

        let mut pb = PathBuilder::new();
        pb.move_to(self.from.x, self.from.y);
        pb.line_to(self.to.x, self.to.y);

        if self.head {
            // Long enough to be seen, but never so long it eats the shaft.
            let head = (self.style.width * HEAD_SCALE).max(HEAD_MIN).min(length * 0.5);
            let dir = Point::from_xy(
                (self.to.x - self.from.x) / length,
                (self.to.y - self.from.y) / length,
            );
            // The barbs run back from the tip, turned to either side.
            let (sin, cos) = HEAD_ANGLE_DEG.to_radians().sin_cos();
            for sign in [1.0, -1.0] {
                let barb = Point::from_xy(
                    -dir.x * cos - sign * -dir.y * sin,
                    sign * -dir.x * sin + -dir.y * cos,
                );
                pb.move_to(self.to.x + barb.x * head, self.to.y + barb.y * head);
                pb.line_to(self.to.x, self.to.y);
            }
        }

        let Some(path) = pb.finish() else {
            return;
        };

        let mut paint = Paint::default();
        paint.set_color(self.style.color);
        paint.anti_alias = true;
        let stroke = Stroke {
            width: self.style.width,
            line_cap: LineCap::Round,
            line_join: LineJoin::Round,
            ..Default::default()
        };
        // One path, one rasterization: a translucent color doesn't darken
        // where the barbs meet the shaft.
        target.stroke_path(&path, &paint, &stroke, Transform::identity(), None);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tiny_skia::{Color, Pixmap};

    fn style() -> Style {
        Style { color: Color::from_rgba8(255, 0, 0, 255), width: 4.0 }
    }

    /// Drags the tool from `from` to `to` and renders what it committed.
    fn drawn(tool: &dyn Tool, from: (f32, f32), to: (f32, f32)) -> Option<Pixmap> {
        let mut gesture = tool.begin(Point::from_xy(from.0, from.1), style());
        gesture.update(Point::from_xy(to.0, to.1));
        let annotation = gesture.finish()?;
        let mut canvas = Pixmap::new(100, 100).unwrap();
        annotation.render(&mut canvas.as_mut());
        Some(canvas)
    }

    fn inked(pixmap: &Pixmap, x: u32, y: u32) -> bool {
        pixmap.pixel(x, y).unwrap().alpha() > 0
    }

    #[test]
    fn a_click_without_a_drag_leaves_nothing() {
        assert!(drawn(&Line, (50.0, 50.0), (50.0, 51.0)).is_none());
        assert!(drawn(&Arrow, (50.0, 50.0), (50.0, 50.0)).is_none());
    }

    #[test]
    fn a_line_connects_the_two_points() {
        let canvas = drawn(&Line, (10.0, 10.0), (90.0, 90.0)).expect("a drag is kept");
        assert!(inked(&canvas, 50, 50), "on the line");
        assert!(inked(&canvas, 11, 11) && inked(&canvas, 89, 89), "to both ends");
        assert!(!inked(&canvas, 50, 20), "away from it");
    }

    #[test]
    fn only_the_arrow_has_barbs_at_its_end() {
        let arrow = drawn(&Arrow, (10.0, 50.0), (90.0, 50.0)).expect("a drag is kept");
        let line = drawn(&Line, (10.0, 50.0), (90.0, 50.0)).expect("a drag is kept");

        // Where a barb reaching back from the tip must be, off the shaft.
        for (x, y) in [(78, 44), (78, 56)] {
            assert!(inked(&arrow, x, y), "arrow barb at {x},{y}");
            assert!(!inked(&line, x, y), "a plain line has none");
        }
        // The head stays near the tip.
        assert!(!inked(&arrow, 30, 44) && !inked(&line, 30, 44));
    }

    #[test]
    fn the_arrowhead_stays_inside_a_short_arrow() {
        let short = drawn(&Arrow, (40.0, 50.0), (60.0, 50.0)).expect("a drag is kept");
        assert!(!inked(&short, 39, 44), "the barbs don't reach past the start");
    }

    #[test]
    fn a_gesture_previews_itself_while_it_is_drawn() {
        let mut gesture = Arrow.begin(Point::from_xy(10.0, 50.0), style());
        gesture.update(Point::from_xy(90.0, 50.0));

        let mut canvas = Pixmap::new(100, 100).unwrap();
        gesture.render(&mut canvas.as_mut());
        assert!(inked(&canvas, 50, 50) && inked(&canvas, 78, 44), "shaft and head");
    }
}
