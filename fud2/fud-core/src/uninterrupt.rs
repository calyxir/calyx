/// Temporarily suppress SIGINT.
///
/// This is an RAII marker that temporarily suppresses the default behavior of
/// the SIGINT signal, i.e., that ignores "control-C." It does not suppress the
/// signal for the entire process group, however, so child processes still get
/// interrupted as usual.
///
/// Use the lifetime of the `Uninterrupt` object to define a scope where this
/// behavior is applied, like this:
///
///     let foo = {
///         let _unint = Uninterrupt::suppress();
///         do_stuff();
///     };
///
/// Then SIGINT will be suppressed during the call to `do_stuff()`.
pub struct Uninterrupt {}

impl Uninterrupt {
    pub fn suppress() -> Self {
        // Register a no-op handler for SIGINT. This is different from ignoring
        // SIGINT altogether because it still allows the signal to go to the
        // children in the process group.
        fn nop() {}
        unsafe {
            libc::signal(libc::SIGINT, nop as usize);
        }
        Self {}
    }
}

impl Drop for Uninterrupt {
    fn drop(&mut self) {
        // Restore the default behavior of SIGINT.
        unsafe {
            libc::signal(libc::SIGINT, libc::SIG_DFL);
        }
    }
}
