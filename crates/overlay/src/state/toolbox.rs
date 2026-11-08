use crate::state::hsv::Hsv;
use ricture_tools::{Setting, Style, Tool, ToolMeta};
use tiny_skia::Color;

const PALETTE: [[u8; 3]; 8] = [
    [0xff, 0x3b, 0x30],
    [0xff, 0x95, 0x00],
    [0xff, 0xcc, 0x00],
    [0x34, 0xc7, 0x59],
    [0x0a, 0x84, 0xff],
    [0xaf, 0x52, 0xde],
    [0xff, 0xff, 0xff],
    [0x00, 0x00, 0x00],
];

/// The registered tools and the user's current choices. The UI only displays
/// this and reports what the user asked to change.
pub(crate) struct Toolbox {
    tools: Vec<Box<dyn Tool>>,
    palette: Vec<Color>,
    /// Index into `tools`, `None` in Select mode.
    active: Option<usize>,
    color: Color,
    /// Stroke width in logical px.
    width: f32,
    /// Part of a width step scrolled so far; touchpads send tiny deltas.
    scrolled: f32,
    picker_open: bool,
    /// What the picker shows; matches `color` once the picker has set it.
    picked: Hsv,
}

impl Toolbox {
    pub(crate) fn new(tools: Vec<Box<dyn Tool>>, color: Color, width: f32) -> Self {
        Self {
            tools,
            palette: PALETTE
                .iter()
                .map(|&[r, g, b]| Color::from_rgba8(r, g, b, 255))
                .collect(),
            active: None,
            color,
            width,
            scrolled: 0.0,
            picker_open: false,
            picked: Hsv::from_color(color),
        }
    }

    pub(crate) fn metas(&self) -> impl Iterator<Item = ToolMeta> + '_ {
        self.tools.iter().map(|tool| tool.meta())
    }

    pub(crate) fn palette(&self) -> &[Color] {
        &self.palette
    }

    pub(crate) fn active_index(&self) -> Option<usize> {
        self.active
    }

    /// The drawing tool in use, `None` in Select mode.
    pub(crate) fn active(&self) -> Option<&dyn Tool> {
        self.tools.get(self.active?).map(|tool| tool.as_ref())
    }

    pub(crate) fn color(&self) -> Color {
        self.color
    }

    pub(crate) fn is_custom_color(&self) -> bool {
        !self.palette.contains(&self.color)
    }

    pub(crate) fn width(&self) -> f32 {
        self.width
    }

    pub(crate) fn is_picker_open(&self) -> bool {
        self.picker_open
    }

    pub(crate) fn picked(&self) -> Hsv {
        self.picked
    }

    /// The picked hue at full saturation and value.
    pub(crate) fn picked_hue_color(&self) -> Color {
        Hsv { saturation: 1.0, value: 1.0, ..self.picked }.to_color(255)
    }

    /// Style for a new gesture; `scale` converts logical px to screenshot px.
    pub(crate) fn style(&self, scale: f32) -> Style {
        Style {
            color: self.color,
            width: self.width * scale,
        }
    }

    /// Switches tool. The width is shared between tools, so it is pulled into
    /// the new tool's range.
    pub(crate) fn select(&mut self, index: Option<usize>) {
        self.active = index.filter(|&index| index < self.tools.len());
        self.picker_open = false;
        if let Some((min, max)) = self.width_range() {
            self.width = self.width.clamp(min, max);
        }
    }

    pub(crate) fn pick_color(&mut self, index: usize) {
        if let Some(&color) = self.palette.get(index) {
            self.color = color;
            self.picker_open = false;
        }
    }

    /// Opening doesn't change the color: the picker starts from the current
    /// one, keeping its hue if the picker itself produced that color.
    pub(crate) fn toggle_picker(&mut self) {
        self.picker_open = !self.picker_open;
        if self.picked.to_color(self.alpha()) != self.color {
            self.picked = Hsv::from_color(self.color);
        }
    }

    pub(crate) fn set_picked_hue(&mut self, hue: f32) {
        self.picked.hue = hue.clamp(0.0, 1.0);
        self.apply_picked();
    }

    pub(crate) fn set_picked_shade(&mut self, saturation: f32, value: f32) {
        self.picked.saturation = saturation.clamp(0.0, 1.0);
        self.picked.value = value.clamp(0.0, 1.0);
        self.apply_picked();
    }

    /// Sets the width to whole pixels within the active tool's range.
    pub(crate) fn set_width(&mut self, value: f32) {
        if let Some((min, max)) = self.width_range() {
            self.width = value.round().clamp(min, max);
        }
    }

    /// Changes the width by `steps` wheel notches, one pixel each.
    pub(crate) fn scroll_width(&mut self, steps: f32) {
        self.scrolled += steps;
        let whole = self.scrolled.trunc();
        if whole != 0.0 {
            self.scrolled -= whole;
            self.set_width(self.width + whole);
        }
    }

    fn alpha(&self) -> u8 {
        self.color.to_color_u8().alpha()
    }

    fn apply_picked(&mut self) {
        self.color = self.picked.to_color(self.alpha());
    }

    fn width_range(&self) -> Option<(f32, f32)> {
        self.active()?.meta().settings.iter().find_map(|setting| match *setting {
            Setting::Width { min, max } => Some((min, max)),
            _ => None,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ricture_tools::Gesture;
    use tiny_skia::Point;

    const START_WIDTH: f32 = 4.0;

    struct Fake {
        settings: &'static [Setting],
    }

    impl Tool for Fake {
        fn meta(&self) -> ToolMeta {
            ToolMeta {
                name: "Fake",
                icon: "",
                settings: self.settings,
            }
        }

        fn begin(&self, _at: Point, _style: Style) -> Box<dyn Gesture> {
            unimplemented!("the toolbox never starts gestures")
        }
    }

    fn toolbox() -> Toolbox {
        Toolbox::new(
            vec![
                Box::new(Fake { settings: &[Setting::Width { min: 1.0, max: 32.0 }] }),
                Box::new(Fake { settings: &[Setting::Width { min: 2.0, max: 8.0 }] }),
                Box::new(Fake { settings: &[Setting::Color] }),
            ],
            Color::from_rgba8(0xff, 0x3b, 0x30, 255),
            START_WIDTH,
        )
    }

    #[test]
    fn starts_in_select_mode() {
        let toolbox = toolbox();
        assert_eq!(toolbox.active_index(), None);
        assert!(toolbox.active().is_none());
    }

    #[test]
    fn out_of_range_tool_falls_back_to_select() {
        let mut toolbox = toolbox();
        toolbox.select(Some(0));
        toolbox.select(Some(99));
        assert_eq!(toolbox.active_index(), None);
    }

    #[test]
    fn width_is_rounded_and_clamped_to_the_active_tool() {
        let mut toolbox = toolbox();
        toolbox.select(Some(0));
        toolbox.set_width(7.6);
        assert_eq!(toolbox.width(), 8.0);
        toolbox.set_width(100.0);
        assert_eq!(toolbox.width(), 32.0);
        toolbox.set_width(-5.0);
        assert_eq!(toolbox.width(), 1.0);
    }

    #[test]
    fn switching_tool_pulls_width_into_its_range() {
        let mut toolbox = toolbox();
        toolbox.select(Some(0));
        toolbox.set_width(20.0);
        toolbox.select(Some(1));
        assert_eq!(toolbox.width(), 8.0);
    }

    #[test]
    fn configured_width_is_clamped_once_a_tool_is_selected() {
        let mut toolbox = Toolbox::new(
            vec![Box::new(Fake { settings: &[Setting::Width { min: 1.0, max: 32.0 }] })],
            Color::BLACK,
            500.0,
        );
        toolbox.select(Some(0));
        assert_eq!(toolbox.width(), 32.0);
    }

    #[test]
    fn width_is_ignored_without_a_width_setting() {
        let mut toolbox = toolbox();
        toolbox.set_width(20.0);
        assert_eq!(toolbox.width(), START_WIDTH);
        toolbox.select(Some(2));
        toolbox.set_width(20.0);
        assert_eq!(toolbox.width(), START_WIDTH);
    }

    #[test]
    fn scrolling_changes_width_by_whole_steps() {
        let mut toolbox = toolbox();
        toolbox.select(Some(0));
        toolbox.scroll_width(2.0);
        assert_eq!(toolbox.width(), START_WIDTH + 2.0);
        toolbox.scroll_width(-1.0);
        assert_eq!(toolbox.width(), START_WIDTH + 1.0);
    }

    #[test]
    fn small_scroll_deltas_add_up() {
        let mut toolbox = toolbox();
        toolbox.select(Some(0));
        toolbox.scroll_width(0.4);
        toolbox.scroll_width(0.4);
        assert_eq!(toolbox.width(), START_WIDTH);
        toolbox.scroll_width(0.4);
        assert_eq!(toolbox.width(), START_WIDTH + 1.0);
    }

    #[test]
    fn scrolling_stops_at_the_range() {
        let mut toolbox = toolbox();
        toolbox.select(Some(1));
        toolbox.scroll_width(100.0);
        assert_eq!(toolbox.width(), 8.0);
        toolbox.scroll_width(-1.0);
        assert_eq!(toolbox.width(), 7.0);
    }

    #[test]
    fn unknown_color_is_ignored() {
        let mut toolbox = toolbox();
        toolbox.pick_color(1);
        let picked = toolbox.color();
        assert_eq!(picked, toolbox.palette()[1]);
        toolbox.pick_color(99);
        assert_eq!(toolbox.color(), picked);
    }

    #[test]
    fn style_scales_width_to_screenshot_pixels() {
        let toolbox = toolbox();
        assert_eq!(toolbox.style(2.0).width, START_WIDTH * 2.0);
    }

    #[test]
    fn opening_the_picker_keeps_the_color() {
        let mut toolbox = toolbox();
        toolbox.pick_color(4);
        let color = toolbox.color();
        toolbox.toggle_picker();
        assert!(toolbox.is_picker_open());
        assert_eq!(toolbox.color(), color);
        assert_eq!(toolbox.picked(), Hsv::from_color(color));
    }

    #[test]
    fn picking_a_shade_makes_the_color_custom() {
        let mut toolbox = toolbox();
        assert!(!toolbox.is_custom_color());
        toolbox.toggle_picker();
        toolbox.set_picked_hue(0.5);
        toolbox.set_picked_shade(0.5, 0.5);
        assert!(toolbox.is_custom_color());
        assert_eq!(toolbox.color(), Color::from_rgba8(64, 128, 128, 255));
    }

    #[test]
    fn picker_values_are_clamped() {
        let mut toolbox = toolbox();
        toolbox.toggle_picker();
        toolbox.set_picked_hue(3.0);
        toolbox.set_picked_shade(-1.0, 2.0);
        let picked = toolbox.picked();
        assert_eq!((picked.hue, picked.saturation, picked.value), (1.0, 0.0, 1.0));
    }

    #[test]
    fn picker_keeps_hue_through_black() {
        let mut toolbox = toolbox();
        toolbox.toggle_picker();
        toolbox.set_picked_hue(0.5);
        toolbox.set_picked_shade(1.0, 0.0);
        toolbox.toggle_picker();
        toolbox.toggle_picker();
        assert_eq!(toolbox.picked().hue, 0.5);
    }

    #[test]
    fn picker_preserves_alpha() {
        let mut toolbox = Toolbox::new(vec![], Color::from_rgba8(10, 20, 30, 128), START_WIDTH);
        toolbox.toggle_picker();
        toolbox.set_picked_hue(0.25);
        assert_eq!(toolbox.color().to_color_u8().alpha(), 128);
    }

    #[test]
    fn choosing_a_palette_color_or_tool_closes_the_picker() {
        let mut toolbox = toolbox();
        toolbox.toggle_picker();
        toolbox.pick_color(0);
        assert!(!toolbox.is_picker_open());
        toolbox.toggle_picker();
        toolbox.select(Some(2));
        assert!(!toolbox.is_picker_open());
    }
}
