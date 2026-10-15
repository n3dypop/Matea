//! Narrow sigaction guard: restore the exact pre-session handlers, including on partial setup.
//! Only one Native session can exist (EXCLUSIVE). No allocation, locks or I/O in the handler.
use std::{
    io,
    mem::MaybeUninit,
    sync::atomic::{AtomicBool, Ordering},
};

static INTERRUPTED: AtomicBool = AtomicBool::new(false);
extern "C" fn on_signal(_: libc::c_int) {
    INTERRUPTED.store(true, Ordering::Relaxed);
}
pub(super) fn interrupted() -> bool {
    INTERRUPTED.load(Ordering::Relaxed)
}

pub(super) struct SignalGuard {
    previous: Vec<(libc::c_int, libc::sigaction)>,
}
impl SignalGuard {
    pub(super) fn install() -> io::Result<Self> {
        INTERRUPTED.store(false, Ordering::Relaxed);
        let mut guard = Self {
            previous: Vec::new(),
        };
        for signal in [libc::SIGINT, libc::SIGTERM] {
            // SAFETY: sigaction is a C record; zero initializes its unused fields.
            // Handler, mask and flags are explicitly initialized before the syscall.
            let mut action: libc::sigaction = unsafe { std::mem::zeroed() };
            action.sa_sigaction = on_signal as *const () as libc::sighandler_t;
            action.sa_flags = libc::SA_RESTART;
            // SAFETY: action.sa_mask is valid writable storage for a sigset_t.
            if unsafe { libc::sigemptyset(&mut action.sa_mask) } != 0 {
                return Err(io::Error::last_os_error());
            }
            let mut old = MaybeUninit::<libc::sigaction>::uninit();
            // SAFETY: valid known signal and initialized action; old points to enough
            // writable storage. The handler is static and only uses a lock-free AtomicBool.
            if unsafe { libc::sigaction(signal, &action, old.as_mut_ptr()) } != 0 {
                return Err(io::Error::last_os_error());
            }
            // SAFETY: successful sigaction initialized old completely.
            guard.previous.push((signal, unsafe { old.assume_init() }));
        }
        Ok(guard)
    }
    pub(super) fn restore(&mut self) -> io::Result<()> {
        let mut failed = Vec::new();
        let mut error = None;
        for (signal, action) in self.previous.drain(..).rev() {
            // SAFETY: action is the kernel-provided record for this valid signal;
            // null oldact requests no output. Session exclusivity prevents our own overlap.
            if unsafe { libc::sigaction(signal, &action, std::ptr::null_mut()) } != 0 {
                error.get_or_insert_with(io::Error::last_os_error);
                failed.push((signal, action));
            }
        }
        self.previous = failed;
        error.map_or(Ok(()), Err)
    }
}
impl Drop for SignalGuard {
    fn drop(&mut self) {
        let _ = self.restore();
    }
}
