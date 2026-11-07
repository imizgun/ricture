use crate::tool::{Annotation, Gesture, Setting, Style, Tool, ToolMeta};
use tiny_skia::{LineCap, LineJoin, Mask, Paint, PathBuilder, PixmapMut, Point, Stroke, Transform};

/// Freehand drawing.
pub struct Brush;

impl Tool for Brush {
    fn meta(&self) -> ToolMeta {
        ToolMeta {
            name: "Brush",
            icon: "M4 20c2.5 0 5-.8 6-3.5l8.5-8.5a2.1 2.1 0 0 0-3-3L7 13.5C4.5 14.5 4 17 4 20z",
            settings: &[Setting::Color, Setting::Width { min: 1.0, max: 32.0 }],
        }
    }

    fn begin(&self, at: Point, style: Style) -> Box<dyn Gesture> {
        Box::new(BrushStroke {
            points: vec![at],
            style,
        })
    }
}

struct BrushStroke {
    points: Vec<Point>,
    style: Style,
}

/// Pointer samples closer than this (in px) are dropped; they add path
/// segments without visible detail.
const MIN_STEP: f32 = 1.0;

impl Gesture for BrushStroke {
    fn update(&mut self, to: Point) {
        let last = *self.points.last().expect("stroke starts with a point");
        if last.distance(to) >= MIN_STEP {
            self.points.push(to);
        }
    }

    fn finish(self: Box<Self>) -> Option<Box<dyn Annotation>> {
        Some(self)
    }
}

impl Annotation for BrushStroke {
    fn render(&self, target: &mut PixmapMut<'_>, clip: Option<&Mask>) {
        let mut paint = Paint::default();
        paint.set_color(self.style.color);
        paint.anti_alias = true;

        let [first, rest @ ..] = self.points.as_slice() else {
            return;
        };

        // A click without movement leaves a round dot.
        if rest.is_empty() {
            if let Some(dot) = PathBuilder::from_circle(first.x, first.y, self.style.width / 2.0) {
                target.fill_path(&dot, &paint, tiny_skia::FillRule::Winding, Transform::identity(), clip);
            }
            return;
        }

        // Smooth the polyline: curve through the midpoints of consecutive
        // samples, using the samples themselves as control points.
        let mut pb = PathBuilder::new();
        pb.move_to(first.x, first.y);
        for pair in self.points.windows(2) {
            let (ctrl, next) = (pair[0], pair[1]);
            pb.quad_to(ctrl.x, ctrl.y, (ctrl.x + next.x) / 2.0, (ctrl.y + next.y) / 2.0);
        }
        let last = self.points[self.points.len() - 1];
        pb.line_to(last.x, last.y);

        let Some(path) = pb.finish() else {
            return;
        };
        let stroke = Stroke {
            width: self.style.width,
            line_cap: LineCap::Round,
            line_join: LineJoin::Round,
            ..Default::default()
        };
        target.stroke_path(&path, &paint, &stroke, Transform::identity(), clip);
    }
}
