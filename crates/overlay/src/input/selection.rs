use tiny_skia::Rect;

#[derive(Default)]
pub(crate) struct Selection {
    pub(crate) start: Option<(f64, f64)>,
    pub(crate) current: Option<(f64, f64)>,
    prev_start: Option<(f64, f64)>,
    prev_current: Option<(f64, f64)>,
    pub(crate) is_dragging: bool,
}

impl Selection {
    pub(crate) fn changed(&self) -> bool {
        (self.start, self.current) != (self.prev_start, self.prev_current)
    }

    pub(crate) fn mark_drawn(&mut self) {
        self.prev_start = self.start;
        self.prev_current = self.current;
    }

    pub(crate) fn rect(&self) -> Option<Rect> {
        Self::rect_from(self.start, self.current)
    }

    /// The rect as it was on the last actual redraw (before `mark_drawn()`),
    /// i.e. what's currently visible on screen.
    pub(crate) fn prev_rect(&self) -> Option<Rect> {
        Self::rect_from(self.prev_start, self.prev_current)
    }

    fn rect_from(start: Option<(f64, f64)>, current: Option<(f64, f64)>) -> Option<Rect> {
        let (Some((x0, y0)), Some((x1, y1))) = (start, current) else {
            return None;
        };
        Rect::from_ltrb(
            x0.min(x1) as f32,
            y0.min(y1) as f32,
            x0.max(x1) as f32,
            y0.max(y1) as f32,
        )
    }

    pub(crate) fn normalized(&self) -> Option<(f64, f64, f64, f64)> {
        let (x0, y0) = self.start?;
        let (x1, y1) = self.current?;
        Some((x0.min(x1), y0.min(y1), (x1 - x0).abs(), (y1 - y0).abs()))
    }
}
