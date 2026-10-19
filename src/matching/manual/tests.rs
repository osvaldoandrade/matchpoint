use super::*;
use ratatui::{backend::TestBackend, Terminal};

fn screen(pager: &Pager, size: (u16, u16)) -> String {
    let mut terminal = Terminal::new(TestBackend::new(size.0, size.1)).unwrap();
    terminal
        .draw(|frame| pager.render(frame, Rect::new(1, 3, size.0 - 2, size.1 - 4)))
        .unwrap();
    terminal
        .backend()
        .buffer()
        .content()
        .iter()
        .map(|cell| cell.symbol())
        .collect()
}
fn press(pager: &mut Pager, code: KeyCode, size: (u16, u16)) {
    assert!(!pager.key(code.into(), size));
}
fn search(pager: &mut Pager, text: &str, size: (u16, u16)) {
    press(pager, KeyCode::Char('/'), size);
    for ch in text.chars() {
        press(pager, KeyCode::Char(ch), size);
    }
    press(pager, KeyCode::Enter, size);
}
#[test]
fn navigation_reaches_every_chapter_and_last_line_at_all_sizes() {
    for size in [(40, 18), (60, 22), (80, 24), (120, 36)] {
        let mut pager = Pager::default();
        for (_, body) in CHAPTERS {
            let first = screen(&pager, size);
            assert!(first.contains("q/Esc back"));
            press(&mut pager, KeyCode::End, size);
            let last = body.split_whitespace().last().unwrap();
            assert!(screen(&pager, size).contains(last), "{size:?}: {last}");
            for code in [
                KeyCode::Home,
                KeyCode::Down,
                KeyCode::Up,
                KeyCode::PageDown,
                KeyCode::PageUp,
                KeyCode::Char(' '),
                KeyCode::Char('b'),
                KeyCode::Char('G'),
                KeyCode::Char('g'),
                KeyCode::Char('j'),
                KeyCode::Char('k'),
            ] {
                press(&mut pager, code, size);
            }
            press(&mut pager, KeyCode::Tab, size);
        }
        assert_eq!(pager.chapter, 0);
        for code in [
            KeyCode::Left,
            KeyCode::Right,
            KeyCode::BackTab,
            KeyCode::Char('l'),
            KeyCode::Char('h'),
        ] {
            press(&mut pager, code, size);
        }
        assert!(pager.key(KeyCode::Char('q').into(), size));
        assert!(pager.key(KeyCode::Esc.into(), size));
    }
}
#[test]
fn search_crosses_chapters_wraps_and_preserves_identifiers_as_inert_text() {
    for size in [(40, 18), (80, 24), (120, 36)] {
        let mut pager = Pager::default();
        search(&mut pager, "SHA-256", size);
        let first = (pager.chapter, pager.offset);
        assert!(screen(&pager, size).contains("SHA-256"));
        press(&mut pager, KeyCode::Char('n'), size);
        assert_ne!((pager.chapter, pager.offset), first);
        press(&mut pager, KeyCode::Char('N'), size);
        assert_eq!((pager.chapter, pager.offset), first);
        let hits = pager
            .matches(geometry(Rect::new(0, 0, size.0 - 2, size.1 - 4)).1.width as usize - 2)
            .len();
        for _ in 0..hits {
            press(&mut pager, KeyCode::Char('n'), size);
        }
        assert_eq!((pager.chapter, pager.offset), first);
        search(&mut pager, "no-such-topic-/run", size);
        assert!(screen(&pager, size).contains("0 matching lines"));
        press(&mut pager, KeyCode::Char('n'), size);
        press(&mut pager, KeyCode::Char('/'), size);
        for _ in 0..130 {
            press(&mut pager, KeyCode::Char('x'), size);
        }
        assert_eq!(pager.query.len(), 128);
        press(&mut pager, KeyCode::Backspace, size);
        press(&mut pager, KeyCode::Esc, size);
        search(&mut pager, "", size);
        press(&mut pager, KeyCode::Char('n'), size);
        search(&mut pager, "keychain", size);
        assert!(screen(&pager, size).to_lowercase().contains("keychain"));
        search(&mut pager, "matching/jobs/JOB_ID/result.txt", size);
        let recovery = CHAPTERS
            .iter()
            .position(|(title, _)| *title == "FILES AND RECOVERY")
            .unwrap();
        assert!(pager
            .matches(36)
            .iter()
            .any(|(chapter, _)| *chapter == recovery));
        search(&mut pager, "FILES AND RECOVERY", size);
        assert_eq!(pager.chapter, recovery);
    }
}
#[test]
fn resize_keeps_the_manual_readable_and_wrapping_never_loses_text() {
    let mut pager = Pager::default();
    for (chapter, (_, body)) in CHAPTERS.iter().enumerate() {
        assert!(body.is_ascii());
        for width in [8, 36, 74, 91, 200] {
            let lines = wrapped(chapter, width);
            assert!(lines.iter().all(|row| row.text.len() <= width));
            assert_eq!(
                lines
                    .iter()
                    .map(|row| row.text.as_str())
                    .collect::<String>()
                    .split_whitespace()
                    .collect::<String>(),
                body.split_whitespace().collect::<String>()
            );
        }
        pager.chapter = chapter;
        press(&mut pager, KeyCode::End, (40, 18));
        assert!(screen(&pager, (120, 36)).contains("q/Esc back"));
    }
    assert!(text(false).contains("matching/jobs/JOB_ID/result.txt"));
    assert!(text(true).starts_with(".TH MATCHPOINT 1"));
    assert!(text(true).contains(".SH TRANSFORMATIONS"));
}
#[test]
fn search_anchor_survives_resize_and_rejects_control_input() {
    let mut pager = Pager::default();
    search(&mut pager, "SHA-256", (40, 18));
    for size in [(120, 36), (40, 18), (80, 24)] {
        assert!(screen(&pager, size).contains("SHA-256"));
    }
    press(&mut pager, KeyCode::Char('/'), (40, 18));
    for ch in ['\n', '\x1b', '\0'] {
        press(&mut pager, KeyCode::Char(ch), (40, 18));
    }
    pager.key(
        KeyEvent::new(KeyCode::Char('s'), KeyModifiers::CONTROL),
        (40, 18),
    );
    assert!(pager.query.is_empty());
    for _ in 0..100 {
        press(&mut pager, KeyCode::Char('é'), (40, 18));
    }
    assert_eq!(pager.query.len(), 128);
    press(&mut pager, KeyCode::Esc, (40, 18));
    for size in [(0, 0), (1, 1), (10, 5)] {
        press(&mut pager, KeyCode::End, size);
    }
}
