unsafe extern "C" {
    fn fork() -> i32;
    fn setsid() -> i32;
}

/// Forks the process and detaches the child from the controlling terminal.
pub(crate) fn daemonize() -> bool {
    // SAFETY: fork() duplicates the whole process, At this point we're still single-threaded
    // and haven't spawned anything, so that's satisfied.
    match unsafe { fork() } {
        -1 => panic!("fork failed"),
        0 => {
            // SAFETY: plain syscall, no preconditions beyond being callable from a single-threaded child
            unsafe { setsid() };
            true
        }
        _ => false,
    }
}
