use std::io::{self, IsTerminal, Write};

use crate::error::Error;

fn prompt(label: &str) -> Result<String, Error> {
    print!("{label}");
    io::stdout().flush()?;
    let mut line = String::new();
    io::stdin().read_line(&mut line)?;
    Ok(line.trim().to_string())
}

pub(super) fn prompt_default(label: &str, default: &str) -> Result<String, Error> {
    let value = prompt(&format!("{label} [{default}]: "))?;
    Ok(if value.is_empty() {
        default.to_string()
    } else {
        value
    })
}

pub fn prompt_hidden(label: &str) -> Result<String, Error> {
    use crossterm::event::{read, Event, KeyCode, KeyEventKind};
    use crossterm::terminal::{disable_raw_mode, enable_raw_mode};
    use zeroize::Zeroizing;
    if !io::stdin().is_terminal() || !io::stdout().is_terminal() {
        return Err(Error::NotATty);
    }
    print!("{label}");
    io::stdout().flush()?;
    enable_raw_mode()?;
    let mut value = Zeroizing::new(String::new());
    let result = loop {
        match read() {
            Ok(Event::Key(key))
                if key.kind == KeyEventKind::Press || key.kind == KeyEventKind::Repeat =>
            {
                match key.code {
                    KeyCode::Enter => break Ok(()),
                    KeyCode::Esc | KeyCode::Char('c')
                        if key.code == KeyCode::Esc
                            || key
                                .modifiers
                                .contains(crossterm::event::KeyModifiers::CONTROL) =>
                    {
                        value.clear();
                        break Err(Error::Msg("cancelled".into()));
                    }
                    KeyCode::Char(ch)
                        if !ch.is_control() && value.len() + ch.len_utf8() <= 8192 =>
                    {
                        value.push(ch)
                    }
                    KeyCode::Backspace => {
                        value.pop();
                    }
                    _ => {}
                }
            }
            Ok(_) => {}
            Err(err) => break Err(err.into()),
        }
    };
    let _ = disable_raw_mode();
    println!();
    result?;
    Ok(std::mem::take(&mut *value))
}
