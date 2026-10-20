use super::{DeviceIo, Ending, TerminalFile};
use matea_render::Size;
use std::{
    io::{self, Read},
    marker::PhantomData,
};

#[derive(Clone)]
pub(super) struct State;
pub(super) struct Native<'fd>(PhantomData<&'fd ()>);
impl<'fd> Native<'fd> {
    pub(super) fn new(_: &'fd (impl TerminalFile + ?Sized)) -> Self {
        Self(PhantomData)
    }
}
impl DeviceIo for Native<'_> {
    type State = State;
    fn state(&self) -> io::Result<State> {
        Err(io::ErrorKind::Unsupported.into())
    }
    fn set_state(&mut self, _: &State) -> io::Result<()> {
        Err(io::ErrorKind::Unsupported.into())
    }
    fn raw_state(&self, _: &State) -> io::Result<State> {
        Err(io::ErrorKind::Unsupported.into())
    }
    fn password_state(&self, _: &State) -> io::Result<State> {
        Err(io::ErrorKind::Unsupported.into())
    }
    fn ending(&self) -> Ending {
        Ending::Lf
    }
    fn size(&self) -> io::Result<Size> {
        Err(io::ErrorKind::Unsupported.into())
    }
}
impl Read for Native<'_> {
    fn read(&mut self, _: &mut [u8]) -> io::Result<usize> {
        Err(io::ErrorKind::Unsupported.into())
    }
}
