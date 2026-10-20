//! Ownership boundaries for backend restoration and terminal exclusivity.
use crate::Backend;
use matea_render::{Size, Surface};
use std::{
    io,
    sync::atomic::{AtomicBool, Ordering},
    time::Duration,
};

pub(super) struct Session<B: Backend> {
    pub(super) backend: B,
    active: bool,
}
impl<B: Backend> Session<B> {
    pub(super) fn enter(backend: B) -> io::Result<Self> {
        let mut session = Self {
            backend,
            active: true,
        };
        if let Err(error) = session.backend.enter() {
            return match session.restore() {
                Ok(()) => Err(error),
                Err(cleanup) => Err(io::Error::other(format!(
                    "{error}; terminal restoration also failed: {cleanup}"
                ))),
            };
        }
        Ok(session)
    }
    pub(super) fn restore(&mut self) -> io::Result<()> {
        if self.active {
            self.backend.restore()?;
            self.active = false;
        }
        Ok(())
    }
}
impl<B: Backend> Drop for Session<B> {
    fn drop(&mut self) {
        let _ = self.restore();
    }
}

pub(super) struct Lease<'a>(&'a AtomicBool);
impl<'a> Lease<'a> {
    pub(super) fn acquire(flag: &'a AtomicBool) -> io::Result<Self> {
        flag.compare_exchange(false, true, Ordering::AcqRel, Ordering::Acquire)
            .map_err(|_| {
                io::Error::new(
                    io::ErrorKind::WouldBlock,
                    "another Matea terminal session is already active",
                )
            })?;
        Ok(Self(flag))
    }
}
impl Drop for Lease<'_> {
    fn drop(&mut self) {
        self.0.store(false, Ordering::Release);
    }
}

/// Field order matters: fully drop the backend and its nested signal resources
/// before releasing the terminal, including after failed cleanup or unwinding.
pub(super) struct Exclusive<'a, B> {
    inner: B,
    _lease: Lease<'a>,
}
impl<'a, B> Exclusive<'a, B> {
    pub(super) fn acquire(
        flag: &'a AtomicBool,
        make: impl FnOnce() -> io::Result<B>,
    ) -> io::Result<Self> {
        let lease = Lease::acquire(flag)?;
        let inner = make()?;
        Ok(Self {
            inner,
            _lease: lease,
        })
    }
}
impl<B: Backend> Backend for Exclusive<'_, B> {
    fn enter(&mut self) -> io::Result<()> {
        self.inner.enter()
    }
    fn restore(&mut self) -> io::Result<()> {
        self.inner.restore()
    }
    fn size(&self) -> io::Result<Size> {
        self.inner.size()
    }
    fn poll(&mut self, timeout: Duration) -> io::Result<Option<crate::TerminalEvent>> {
        self.inner.poll(timeout)
    }
    fn draw(&mut self, frame: &Surface, previous: Option<&Surface>) -> io::Result<()> {
        self.inner.draw(frame, previous)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::cell::RefCell;
    #[test]
    fn exclusive_lease_outlives_backend_body_and_nested_destructors() {
        struct Cleanup<'a>(&'a AtomicBool, &'a RefCell<Vec<&'static str>>);
        impl Drop for Cleanup<'_> {
            fn drop(&mut self) {
                assert!(
                    self.0.load(Ordering::Acquire),
                    "nested cleanup lost terminal ownership"
                );
                self.1.borrow_mut().push("signal field cleanup");
            }
        }
        struct BackendFields<'a> {
            _cleanup: Cleanup<'a>,
            events: &'a RefCell<Vec<&'static str>>,
        }
        impl Drop for BackendFields<'_> {
            fn drop(&mut self) {
                self.events
                    .borrow_mut()
                    .push("backend cleanup failed; fields still need Drop");
            }
        }
        let flag = AtomicBool::new(false);
        let events = RefCell::new(Vec::new());
        let owned = Exclusive::acquire(&flag, || {
            Ok(BackendFields {
                _cleanup: Cleanup(&flag, &events),
                events: &events,
            })
        })
        .unwrap();
        assert!(flag.load(Ordering::Acquire));
        let rejected = Exclusive::acquire(&flag, || -> io::Result<()> {
            panic!("overlapping constructor ran")
        });
        assert!(matches!(rejected, Err(e) if e.kind() == io::ErrorKind::WouldBlock));
        drop(owned);
        assert_eq!(
            *events.borrow(),
            [
                "backend cleanup failed; fields still need Drop",
                "signal field cleanup"
            ]
        );
        assert!(!flag.load(Ordering::Acquire));
        assert!(Exclusive::acquire(&flag, || Ok(())).is_ok());
    }
    #[test]
    fn failed_and_panicking_constructors_release_their_owned_lease() {
        let flag = AtomicBool::new(false);
        let failed = Exclusive::<()>::acquire(&flag, || {
            Err(io::Error::new(
                io::ErrorKind::BrokenPipe,
                "constructor fixture",
            ))
        });
        assert!(matches!(failed, Err(e) if e.kind() == io::ErrorKind::BrokenPipe));
        assert!(!flag.load(Ordering::Acquire));
        let panic = std::panic::catch_unwind(|| {
            Exclusive::<()>::acquire(&flag, || panic!("constructor fixture"))
        });
        assert!(panic.is_err());
        assert!(!flag.load(Ordering::Acquire));
    }
}
