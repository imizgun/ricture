//! Startup and per-frame timing plus memory use, printed as a table on exit.
//!
//! Everything here is a no-op unless the `enabled` feature is on, which the
//! `ricture` binary exposes as `profiling`:
//!
//! ```sh
//! cargo prof            # alias, see .cargo/config.toml
//! ```
//!
//! - [`mark`] records a point on the startup timeline.
//! - [`span`] measures a stage until the guard drops. Repeated spans with the
//!   same label (per-frame work) are aggregated; the first one also goes on
//!   the timeline, since it is the one startup pays for.
//! - [`report`] prints everything to stderr.
//! - [`exit_after_first_frame`] (env `RICTURE_PROFILE_EXIT=1`) closes the
//!   overlay right after its first frame, for repeatable startup runs.

#[cfg(feature = "enabled")]
mod imp;

#[cfg(feature = "enabled")]
pub use imp::{Span, exit_after_first_frame, mark, report, span};

#[cfg(not(feature = "enabled"))]
mod noop {
    pub struct Span;

    impl Span {
        #[inline(always)]
        pub fn cancel(self) {}
    }

    #[inline(always)]
    pub fn mark(_label: &'static str) {}

    #[inline(always)]
    pub fn span(_label: &'static str) -> Span {
        Span
    }

    #[inline(always)]
    pub fn report() {}

    #[inline(always)]
    pub fn exit_after_first_frame() -> bool {
        false
    }
}

#[cfg(not(feature = "enabled"))]
pub use noop::{Span, exit_after_first_frame, mark, report, span};
