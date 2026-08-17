/// What the user asked for on exit: `run()` reports this back so the caller
/// knows whether to write `save_path` to disk or hand it to the clipboard.
pub enum Action {
    Copy,
    Save,
}
