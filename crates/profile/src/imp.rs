use std::collections::HashMap;
use std::fmt::Write as _;
use std::io::IsTerminal;
use std::sync::{LazyLock, Mutex};
use std::time::{Duration, Instant};

static STATE: LazyLock<Mutex<State>> = LazyLock::new(|| {
    Mutex::new(State {
        start: Instant::now(),
        timeline: Vec::new(),
        spans: Vec::new(),
        span_index: HashMap::new(),
    })
});

struct State {
    start: Instant,
    timeline: Vec<Event>,
    /// In order of first use.
    spans: Vec<SpanStats>,
    span_index: HashMap<&'static str, usize>,
}

struct Event {
    label: &'static str,
    /// Since the first profiling call.
    at: Duration,
    /// Set for a span: how long it took.
    took: Option<Duration>,
    rss: u64,
}

struct SpanStats {
    label: &'static str,
    durations: Vec<Duration>,
}

/// Records a point on the startup timeline.
pub fn mark(label: &'static str) {
    let rss = rss_bytes();
    let mut state = lock();
    let at = state.start.elapsed();
    state.timeline.push(Event { label, at, took: None, rss });
}

/// Measures from now until the returned guard drops.
pub fn span(label: &'static str) -> Span {
    // Touch the state first so its creation isn't measured.
    drop(lock());
    Span { label, start: Instant::now() }
}

pub struct Span {
    label: &'static str,
    start: Instant,
}

impl Span {
    /// Drops the measurement, e.g. when the stage turned out to have nothing
    /// to do.
    pub fn cancel(self) {
        std::mem::forget(self);
    }
}

/// Whether the overlay should close right after its first frame.
pub fn exit_after_first_frame() -> bool {
    std::env::var_os("RICTURE_PROFILE_EXIT").is_some_and(|v| v == "1")
}

impl Drop for Span {
    fn drop(&mut self) {
        let took = self.start.elapsed();
        let rss = rss_bytes();
        let mut state = lock();
        let state = &mut *state;
        let index = match state.span_index.get(self.label) {
            Some(&index) => index,
            None => {
                let at = self.start.duration_since(state.start) + took;
                state.timeline.push(Event { label: self.label, at, took: Some(took), rss });
                state.spans.push(SpanStats { label: self.label, durations: Vec::new() });
                state.span_index.insert(self.label, state.spans.len() - 1);
                state.spans.len() - 1
            }
        };
        state.spans[index].durations.push(took);
    }
}

fn lock() -> std::sync::MutexGuard<'static, State> {
    STATE.lock().unwrap_or_else(|poisoned| poisoned.into_inner())
}

/// Resident set size, from the cheap `/proc/self/statm`.
fn rss_bytes() -> u64 {
    let Ok(statm) = std::fs::read_to_string("/proc/self/statm") else { return 0 };
    let pages: u64 = statm.split_whitespace().nth(1).and_then(|p| p.parse().ok()).unwrap_or(0);
    pages * 4096
}

/// Peak RSS so far.
fn peak_rss_bytes() -> u64 {
    std::fs::read_to_string("/proc/self/status")
        .ok()
        .and_then(|status| {
            let line = status.lines().find(|l| l.starts_with("VmHWM:"))?;
            line.split_whitespace().nth(1)?.parse::<u64>().ok()
        })
        .map_or(0, |kib| kib * 1024)
}

/// Prints the timeline and span statistics to stderr.
pub fn report() {
    let state = lock();
    let style = Style::new(std::io::stderr().is_terminal());
    eprint!("{}", render(&state, peak_rss_bytes(), &style));
}

struct Style {
    color: bool,
}

impl Style {
    fn new(color: bool) -> Self {
        Self { color }
    }

    fn paint(&self, code: &str, text: &str) -> String {
        if self.color { format!("\x1b[{code}m{text}\x1b[0m") } else { text.to_string() }
    }

    fn bold(&self, text: &str) -> String {
        self.paint("1", text)
    }

    fn dim(&self, text: &str) -> String {
        self.paint("2", text)
    }

    /// Green under 4 ms, yellow under 16 ms (a 60 Hz frame), red above.
    fn time(&self, d: Duration, text: &str) -> String {
        let ms = d.as_secs_f64() * 1e3;
        let code = if ms < 4.0 { "32" } else if ms < 16.0 { "33" } else { "31" };
        self.paint(code, text)
    }
}

const BAR_WIDTH: usize = 24;
const LABEL_WIDTH: usize = 30;

fn render(state: &State, peak: u64, style: &Style) -> String {
    let mut out = String::new();
    let total = state.timeline.last().map_or(Duration::ZERO, |e| e.at);

    let _ = writeln!(out, "\n{}", style.bold("ricture profile — startup timeline"));
    let _ = writeln!(
        out,
        "{}",
        style.dim(&format!(
            "  {:<LABEL_WIDTH$} {:>9} {:>9}  {:<BAR_WIDTH$} {:>9} {:>9}",
            "stage", "at", "took", "", "rss", "Δrss"
        ))
    );
    let mut prev_at = Duration::ZERO;
    let mut prev_rss = state.timeline.first().map_or(0, |e| e.rss);
    for event in &state.timeline {
        // A mark took the time since the previous event; a span knows its own.
        let took = event.took.unwrap_or(event.at.saturating_sub(prev_at));
        let label = match event.took {
            Some(_) => format!("  {}", event.label),
            None => event.label.to_string(),
        };
        let delta = event.rss as i64 - prev_rss as i64;
        let _ = writeln!(
            out,
            "  {:<LABEL_WIDTH$} {:>9} {}  {} {:>9} {:>9}",
            truncate(&label, LABEL_WIDTH),
            ms(event.at),
            style.time(took, &format!("{:>9}", ms(took))),
            bar(took, total),
            mib(event.rss),
            style.dim(&signed_mib(delta)),
        );
        prev_at = event.at;
        prev_rss = event.rss;
    }
    let _ = writeln!(out, "  {:<LABEL_WIDTH$} {:>9}", style.bold("total"), style.bold(&ms(total)));
    let _ = writeln!(out, "  {:<LABEL_WIDTH$} {:>9}", "peak rss", mib(peak));

    let repeated: Vec<_> = state.spans.iter().filter(|s| s.durations.len() > 1).collect();
    if !repeated.is_empty() {
        let _ = writeln!(out, "\n{}", style.bold("per-frame stages"));
        let _ = writeln!(
            out,
            "{}",
            style.dim(&format!(
                "  {:<LABEL_WIDTH$} {:>6} {:>9} {:>9} {:>9} {:>9}",
                "stage", "calls", "first", "median", "p95", "max"
            ))
        );
        for stats in repeated {
            let mut sorted = stats.durations.clone();
            sorted.sort();
            let first = stats.durations[0];
            let median = percentile(&sorted, 0.5);
            let p95 = percentile(&sorted, 0.95);
            let max = *sorted.last().unwrap();
            let _ = writeln!(
                out,
                "  {:<LABEL_WIDTH$} {:>6} {} {} {} {}",
                truncate(stats.label, LABEL_WIDTH),
                sorted.len(),
                style.time(first, &format!("{:>9}", ms(first))),
                style.time(median, &format!("{:>9}", ms(median))),
                style.time(p95, &format!("{:>9}", ms(p95))),
                style.time(max, &format!("{:>9}", ms(max))),
            );
        }
    }
    out.push('\n');
    out
}

fn percentile(sorted: &[Duration], p: f64) -> Duration {
    let index = ((sorted.len() - 1) as f64 * p).round() as usize;
    sorted[index]
}

fn bar(part: Duration, total: Duration) -> String {
    let ratio = if total.is_zero() { 0.0 } else { part.as_secs_f64() / total.as_secs_f64() };
    let eighths = (ratio.clamp(0.0, 1.0) * (BAR_WIDTH * 8) as f64).round() as usize;
    const PARTIAL: [char; 8] = [' ', '▏', '▎', '▍', '▌', '▋', '▊', '▉'];
    let mut bar = "█".repeat(eighths / 8);
    if eighths % 8 != 0 {
        bar.push(PARTIAL[eighths % 8]);
    }
    format!("{bar:<BAR_WIDTH$}")
}

fn ms(d: Duration) -> String {
    format!("{:.1} ms", d.as_secs_f64() * 1e3)
}

fn mib(bytes: u64) -> String {
    format!("{:.1} MB", bytes as f64 / (1024.0 * 1024.0))
}

fn signed_mib(bytes: i64) -> String {
    if bytes == 0 {
        return String::new();
    }
    format!("{:+.1} MB", bytes as f64 / (1024.0 * 1024.0))
}

fn truncate(text: &str, width: usize) -> String {
    if text.chars().count() <= width {
        return text.to_string();
    }
    let mut cut: String = text.chars().take(width - 1).collect();
    cut.push('…');
    cut
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn report_lists_marks_spans_and_frame_stats() {
        let mut state = State {
            start: Instant::now(),
            timeline: Vec::new(),
            spans: Vec::new(),
            span_index: HashMap::new(),
        };
        let mb = 1024 * 1024;
        state.timeline.push(Event { label: "capture", at: ms_(20), took: None, rss: 6 * mb });
        state.timeline.push(Event { label: "draw", at: ms_(50), took: Some(ms_(30)), rss: 30 * mb });
        state.spans.push(SpanStats { label: "draw", durations: vec![ms_(30), ms_(2), ms_(3)] });

        let text = render(&state, 31 * mb, &Style::new(false));
        assert!(text.contains("capture"), "{text}");
        assert!(text.contains("  draw"), "{text}");
        assert!(text.contains("+24.0 MB"), "{text}");
        assert!(text.contains("peak rss"), "{text}");
        assert!(text.contains("per-frame stages"), "{text}");
        // calls, first, median
        assert!(text.contains("     3   30.0 ms    3.0 ms"), "{text}");
    }

    #[test]
    fn bar_is_proportional_and_fixed_width() {
        assert_eq!(bar(ms_(50), ms_(100)).chars().filter(|&c| c == '█').count(), BAR_WIDTH / 2);
        assert_eq!(bar(ms_(1), ms_(100)).chars().count(), BAR_WIDTH);
        assert_eq!(bar(ms_(1), Duration::ZERO).trim(), "");
    }

    fn ms_(n: u64) -> Duration {
        Duration::from_millis(n)
    }
}
