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
    previous: Journal<(libc::c_int, libc::sigaction)>,
}
/// A fixed two-signal journal. Successful entries are removed in place; failed
/// entries retain their exact action for retry, without allocating in cleanup.
struct Journal<T> {
    slots: [Option<T>; 2],
}
impl<T> Journal<T> {
    fn restore_with(&mut self, mut restore: impl FnMut(&T) -> io::Result<()>) -> io::Result<()> {
        let mut error = None;
        for slot in self.slots.iter_mut().rev() {
            if let Some(action) = slot {
                match restore(action) {
                    Ok(()) => *slot = None,
                    Err(e) => {
                        error.get_or_insert(e);
                    }
                }
            }
        }
        error.map_or(Ok(()), Err)
    }
}
impl SignalGuard {
    pub(super) fn install() -> io::Result<Self> {
        INTERRUPTED.store(false, Ordering::Relaxed);
        let mut guard = Self {
            previous: Journal {
                slots: [None, None],
            },
        };
        for (slot, signal) in guard
            .previous
            .slots
            .iter_mut()
            .zip([libc::SIGINT, libc::SIGTERM])
        {
            // SAFETY: sigaction is a C record; zero initializes its unused fields.
            // Handler, mask and flags are explicitly initialized before the syscall.
            let mut action: libc::sigaction = unsafe { std::mem::zeroed() };
            action.sa_sigaction = on_signal as *const () as libc::sighandler_t;
            action.sa_flags = libc::SA_RESTART;
            // SAFETY: action.sa_mask is valid writable storage for a sigset_t.
            if unsafe { libc::sigemptyset(&mut action.sa_mask) } != 0 {
                return Err(io::Error::last_os_error());
            }
            // Initialize unused fields too; only successful calls are consumed.
            let mut old = MaybeUninit::<libc::sigaction>::zeroed();
            // SAFETY: valid known signal and initialized action; old points to enough
            // writable storage. The handler is static and only uses a lock-free AtomicBool.
            if unsafe { libc::sigaction(signal, &action, old.as_mut_ptr()) } != 0 {
                return Err(io::Error::last_os_error());
            }
            // SAFETY: successful sigaction initialized old completely.
            *slot = Some((signal, unsafe { old.assume_init() }));
        }
        Ok(guard)
    }
    pub(super) fn restore(&mut self) -> io::Result<()> {
        self.previous.restore_with(|(signal, action)| {
            // SAFETY: action is the kernel-provided record for this valid signal;
            // null oldact requests no output. Session exclusivity prevents our own overlap.
            if unsafe { libc::sigaction(*signal, action, std::ptr::null_mut()) } != 0 {
                Err(io::Error::last_os_error())
            } else {
                Ok(())
            }
        })
    }
}

impl Drop for SignalGuard {
    fn drop(&mut self) {
        let _ = self.restore();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn restoration_retries_only_failed_records_in_reverse_install_order() {
        for failures in 0..4 {
            let mut journal = Journal {
                slots: [Some(0), Some(1)],
            };
            let mut calls = Vec::new();
            let result = journal.restore_with(|&id| {
                calls.push(id);
                if failures & (1 << id) != 0 {
                    Err(io::Error::new(io::ErrorKind::PermissionDenied, "fixture"))
                } else {
                    Ok(())
                }
            });
            assert_eq!(calls, [1, 0]);
            assert_eq!(result.is_err(), failures != 0);
            if let Err(error) = result {
                assert_eq!(error.kind(), io::ErrorKind::PermissionDenied);
            }
            calls.clear();
            journal
                .restore_with(|&id| {
                    calls.push(id);
                    Ok(())
                })
                .unwrap();
            let expected: Vec<_> = [1, 0]
                .into_iter()
                .filter(|id| failures & (1 << id) != 0)
                .collect();
            assert_eq!(calls, expected);
            journal
                .restore_with(|_| panic!("successful action restored twice"))
                .unwrap();
        }
    }
    #[test]
    fn partial_installation_journal_never_restores_unacquired_signals() {
        for slots in [[None, None], [Some(7), None]] {
            let expected = slots.iter().rev().flatten().copied().collect::<Vec<_>>();
            let mut journal = Journal { slots };
            let mut actual = Vec::new();
            journal
                .restore_with(|&record| {
                    actual.push(record);
                    Ok(())
                })
                .unwrap();
            assert_eq!(actual, expected);
        }
    }
}
