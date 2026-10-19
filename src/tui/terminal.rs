use std::io::{self, Stdout};

use crossterm::event::{DisableBracketedPaste, EnableBracketedPaste};
use crossterm::terminal::{
    disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen,
};
use crossterm::{execute, ExecutableCommand};

use crate::error::Error;

pub(crate) struct CrosstermTerm {
    _stdout: Stdout,
    active: bool,
}

impl CrosstermTerm {
    pub(crate) fn enter() -> Result<Self, Error> {
        let mut stdout = io::stdout();
        enable_raw_mode()?;
        execute!(stdout, EnterAlternateScreen, EnableBracketedPaste)?;
        let _ = stdout.execute(crossterm::cursor::Hide);
        Ok(Self {
            _stdout: stdout,
            active: true,
        })
    }

    pub(crate) fn restore() -> io::Result<()> {
        let mut stdout = io::stdout();
        let _ = disable_raw_mode();
        execute!(stdout, DisableBracketedPaste, LeaveAlternateScreen)?;
        let _ = stdout.execute(crossterm::cursor::Show);
        Ok(())
    }

    pub(crate) fn leave(&mut self) -> Result<(), Error> {
        if self.active {
            Self::restore()?;
            self.active = false;
        }
        Ok(())
    }
}

impl Drop for CrosstermTerm {
    fn drop(&mut self) {
        let _ = self.leave();
    }
}

pub trait TermMode {
    fn enter(&mut self) -> io::Result<()>;
    fn leave(&mut self) -> io::Result<()>;
}

pub struct Guard<T: TermMode> {
    inner: T,
    active: bool,
}

impl<T: TermMode> Guard<T> {
    pub fn enter(mut inner: T) -> io::Result<Self> {
        inner.enter()?;
        Ok(Self {
            inner,
            active: true,
        })
    }

    pub fn leave(&mut self) -> io::Result<()> {
        if self.active {
            self.inner.leave()?;
            self.active = false;
        }
        Ok(())
    }
}

impl<T: TermMode> Drop for Guard<T> {
    fn drop(&mut self) {
        let _ = self.leave();
    }
}
