use super::*;
use crate::repl::sensitive;
use ratatui::backend::TestBackend;
use ratatui::Terminal;

struct Fake {
    log: Vec<&'static str>,
}

impl TermMode for Fake {
    fn enter(&mut self) -> io::Result<()> {
        self.log.push("enter");
        Ok(())
    }
    fn leave(&mut self) -> io::Result<()> {
        self.log.push("leave");
        Ok(())
    }
}

#[test]
fn guard_restores_once() {
    let mut guard = Guard::enter(Fake { log: Vec::new() }).unwrap();
    guard.leave().unwrap();
    guard.leave().unwrap();
    drop(guard);
}

#[test]
fn render_survives_small_and_large_terminals() {
    let model = Model::new();
    for (w, h) in [(20, 5), (40, 10), (80, 24), (120, 40)] {
        let backend = TestBackend::new(w, h);
        let mut terminal = Terminal::new(backend).unwrap();
        terminal.draw(|frame| render(frame, &model)).unwrap();
    }
}

#[test]
fn standard_terminal_shows_selected_job_counts() {
    let mut model = Model::new();
    model.home = false;
    let id = "01234567-0123-4567-8901-234567890123".to_string();
    model.selected_job = Some(id.clone());
    model.jobs.push(JobStatus {
        id,
        state: "RUNNING".into(),
        desired: "run".into(),
        counts: crate::domain::Counts {
            pending: 80,
            succeeded: 19,
            unknown: 1,
            ..Default::default()
        },
        batches: 1,
    });
    let rendered = screen(&model, 120, 24);
    assert!(rendered.contains("Uncertain 1"));
    assert!(rendered.contains("20%"));
}

fn large_job_model(state: &str, total: i64, viewport: (u16, u16)) -> Model {
    let mut model = Model::new();
    model.home = false;
    model.panel = 3;
    model.viewport = viewport;
    let id = "01234567-0123-4567-8901-234567890123".to_string();
    model.selected_job = Some(id.clone());
    let mut counts = crate::domain::Counts::default();
    match state {
        "RUNNING" => {
            counts.succeeded = total / 2;
            counts.pending = total - counts.succeeded;
        }
        "COMPLETED_WITH_ERRORS" => {
            counts.succeeded = total - 1;
            counts.failed_final = 1;
        }
        "BLOCKED_UNKNOWN" => counts.unknown = total,
        _ => counts.succeeded = total,
    }
    model.jobs.push(JobStatus {
        id,
        state: state.into(),
        desired: "run".into(),
        counts,
        batches: 25,
    });
    model
}

#[test]
fn large_job_summary_preserves_full_counts_in_all_supported_viewports() {
    for state in [
        "RUNNING",
        "COMPLETED",
        "COMPLETED_WITH_ERRORS",
        "BLOCKED_UNKNOWN",
    ] {
        for total in [1_000_000, 20_000_000, i64::MAX] {
            for (w, h) in [(40, 18), (80, 24), (100, 30), (120, 40)] {
                let model = large_job_model(state, total, (w, h));
                let counts = &model.jobs[0].counts;
                let done = counts.succeeded + counts.failed_final + counts.unknown;
                let rendered = screen(&model, w, h);
                assert!(
                    rendered.contains(&total.to_string()),
                    "{state} {w}x{h}: {total}"
                );
                assert!(
                    rendered.contains(&done.to_string()),
                    "{state} {w}x{h}: {done}"
                );
                assert!(
                    rendered.contains(view::state_label(state)),
                    "{state} {w}x{h}"
                );
                if total <= 20_000_000 {
                    assert!(
                        rendered.contains(&format!("{done}/{total}")),
                        "{state} {w}x{h}"
                    );
                }
            }
        }
    }
}

fn sample_batches() -> Vec<crate::store::BatchInfo> {
    (1..=25)
        .map(|batch_id| crate::store::BatchInfo {
            batch_id,
            state: "COMPLETED".into(),
            row_from: batch_id,
            row_to: batch_id,
            pending: 0,
            in_flight: 0,
            retry_wait: 0,
            succeeded: 1,
            failed: 0,
            unknown: 0,
        })
        .collect()
}

#[test]
fn batch_paging_accounts_for_wrapped_large_job_summary() {
    let batches = sample_batches();
    for state in ["COMPLETED", "COMPLETED_WITH_ERRORS", "BLOCKED_UNKNOWN"] {
        for total in [1_000_000, i64::MAX] {
            for (w, h) in [(40, 18), (80, 24), (100, 30), (120, 40)] {
                let mut model = large_job_model(state, total, (w, h));
                model.panel = 2;
                let mut seen = std::collections::BTreeSet::new();
                while model.batch_offset < 25 {
                    model.batch_items = batches
                        .iter()
                        .skip(model.batch_offset as usize)
                        .take(view::batch_page_size(&model) as usize)
                        .cloned()
                        .collect();
                    let rendered = screen(&model, w, h);
                    for n in 1..=25 {
                        if rendered.contains(&format!(" {n:<4} ")) {
                            seen.insert(n);
                        }
                    }
                    apply_key(&mut model, key(KeyCode::PageDown));
                }
                assert_eq!(seen.len(), 25, "{state} {total} {w}x{h}: {seen:?}");
            }
        }
    }
}

#[test]
fn compact_export_error_keeps_counts_and_detail_rows_accessible() {
    let batches = sample_batches();
    for total in [1_000_000, 20_000_000, i64::MAX] {
        for (w, h) in [(40, 18), (80, 24), (100, 30), (120, 40)] {
            let mut model = large_job_model("BLOCKED_UNKNOWN", total, (w, h));
            model.show_error(&Error::UnknownBlocks { unknown: total });
            model.events = (0..10).map(|n| format!("event-marker-{n:02}")).collect();
            let mut events_seen = std::collections::BTreeSet::new();
            for _ in 0..15 {
                let rendered = screen(&model, w, h);
                if w == 40 && total == i64::MAX {
                    assert!(rendered.contains(&format!("Done {total}")));
                    assert!(rendered.contains(&format!("Total {total}")));
                } else {
                    assert!(rendered.contains(&format!("{total}/{total}")));
                }
                for n in 0..10 {
                    if rendered.contains(&format!("event-marker-{n:02}")) {
                        events_seen.insert(n);
                    }
                }
                apply_key(&mut model, key(KeyCode::PageDown));
            }
            assert_eq!(events_seen.len(), 10, "{total} {w}x{h}: {events_seen:?}");
            model.panel = 2;
            model.scroll = 0;
            let mut batches_seen = std::collections::BTreeSet::new();
            while model.batch_offset < 25 {
                model.batch_items = batches
                    .iter()
                    .skip(model.batch_offset as usize)
                    .take(view::batch_page_size(&model) as usize)
                    .cloned()
                    .collect();
                let rendered = screen(&model, w, h);
                for n in 1..=25 {
                    if rendered.contains(&format!(" {n:<4} ")) {
                        batches_seen.insert(n);
                    }
                }
                apply_key(&mut model, key(KeyCode::PageDown));
            }
            assert_eq!(batches_seen.len(), 25, "{total} {w}x{h}: {batches_seen:?}");
        }
    }
}

#[test]
fn repl_key_completes_and_hides_secrets() {
    let mut model = Model::new();
    model.repl = "job l".into();
    apply_key(&mut model, key(KeyCode::Tab));
    assert_eq!(model.repl, "job list");
    model.repl = "client_secret=abc".into();
    let line = apply_key(&mut model, key(KeyCode::Enter)).unwrap();
    assert!(sensitive(&line));
    assert!(model.history.is_empty());
}

pub(super) fn key(code: KeyCode) -> KeyEvent {
    KeyEvent::new(code, KeyModifiers::NONE)
}

pub(super) fn screen(model: &Model, w: u16, h: u16) -> String {
    let mut terminal = Terminal::new(TestBackend::new(w, h)).unwrap();
    terminal.draw(|frame| render(frame, model)).unwrap();
    terminal
        .backend()
        .buffer()
        .content()
        .iter()
        .map(|cell| cell.symbol())
        .collect()
}

#[tokio::test]
async fn configuration_form_masks_secret_cancels_and_saves_without_history() {
    use std::sync::Arc;
    let dir = tempfile::tempdir().unwrap();
    let app = App::open(
        dir.path().join("data"),
        Arc::new(crate::secrets::MemorySecrets::new()),
        Arc::new(crate::timeutil::SystemClock),
    )
    .unwrap();
    let mut model = Model::new();
    for save in [false, true] {
        run_repl_line(&app, &mut model, "configure").await.unwrap();
        apply_key(&mut model, key(KeyCode::Tab));
        for ch in "test-client".chars() {
            apply_key(&mut model, key(KeyCode::Char(ch)));
        }
        for url in ["https://example.test/data", "https://example.test/token"] {
            apply_key(&mut model, key(KeyCode::Tab));
            apply_paste(&mut model, url);
        }
        apply_key(&mut model, key(KeyCode::F(2)));
        apply_key(&mut model, key(KeyCode::Tab));
        for ch in "test-only-secret".chars() {
            apply_key(&mut model, key(KeyCode::Char(ch)));
        }
        for (w, h) in [(20, 5), (40, 18), (80, 24), (120, 40)] {
            let rendered = screen(&model, w, h);
            assert!(!rendered.contains("test-only-secret"));
            if w >= 40 {
                assert!(rendered.contains("****************"));
            }
        }
        if save {
            apply_key(
                &mut model,
                KeyEvent::new(KeyCode::F(2), KeyModifiers::SHIFT),
            );
            for _ in 0..4 {
                apply_key(&mut model, key(KeyCode::Tab));
            }
            apply_key(&mut model, key(KeyCode::Char(' ')));
            let action = apply_key(
                &mut model,
                KeyEvent::new(KeyCode::Char('s'), KeyModifiers::CONTROL),
            )
            .unwrap();
            run_repl_line(&app, &mut model, &action).await.unwrap();
        } else {
            apply_key(&mut model, key(KeyCode::Esc));
            assert!(model.settings.is_none());
            assert!(!app.paths.config_file().exists());
        }
    }
    assert!(app.api_config().unwrap().remote_enabled);
    assert!(model.api_ready);
    assert!(model.history.is_empty());
    assert!(model.events.iter().all(|e| !e.contains("test-only-secret")));
    assert!(!std::fs::read_to_string(app.paths.config_file())
        .unwrap()
        .contains("test-only-secret"));
}

#[tokio::test]
async fn invalid_form_keeps_input_and_running_job_blocks_credential_changes() {
    use std::sync::Arc;
    let dir = tempfile::tempdir().unwrap();
    let app = App::open(
        dir.path().join("data"),
        Arc::new(crate::secrets::MemorySecrets::new()),
        Arc::new(crate::timeutil::SystemClock),
    )
    .unwrap();
    let mut model = Model::new();
    run_repl_line(&app, &mut model, "configure").await.unwrap();
    assert!(run_repl_line(&app, &mut model, "save-api").await.is_err());
    assert!(model.settings.is_some());
    assert!(!app.paths.config_file().exists());
    apply_key(&mut model, key(KeyCode::Esc));
    model.busy = true;
    run_repl_line(&app, &mut model, "configure").await.unwrap();
    assert!(run_repl_line(&app, &mut model, "save-settings")
        .await
        .is_err());
    apply_key(&mut model, key(KeyCode::Esc));
    assert!(run_repl_line(&app, &mut model, "auth configure")
        .await
        .is_err());
    assert!(model.settings.is_none());
}

#[test]
fn secret_commands_are_not_rendered_and_help_is_keyboard_accessible() {
    let mut model = Model::new();
    model.repl = "auth configure --client-secret=test-only-secret".into();
    for (w, h) in [(20, 5), (80, 24), (120, 40)] {
        assert!(!screen(&model, w, h).contains("test-only-secret"));
    }
    apply_key(&mut model, key(KeyCode::F(1)));
    assert!(screen(&model, 120, 40).contains("Settings"));
    apply_key(&mut model, key(KeyCode::F(1)));
    assert!(!model.help);
}

#[tokio::test]
async fn command_results_stay_in_ui_and_selected_job_is_persisted() {
    use std::sync::Arc;
    let dir = tempfile::tempdir().unwrap();
    let app = App::open(
        dir.path().join("data"),
        Arc::new(crate::secrets::MemorySecrets::new()),
        Arc::new(crate::timeutil::SystemClock),
    )
    .unwrap();
    let input = dir.path().join("in.csv");
    std::fs::write(&input, "52998224725\n").unwrap();
    let first = app
        .create_job(&input, crate::jobcfg::default_tuning(), None)
        .unwrap();
    let second = app
        .create_job(&input, crate::jobcfg::default_tuning(), None)
        .unwrap();
    let mut model = Model::new();
    model.selected_job = Some(first.clone());
    refresh(&app, &mut model).await.unwrap();
    let command = apply_key(&mut model, key(KeyCode::F(5))).unwrap();
    assert_eq!(
        run_repl_line(&app, &mut model, &command).await.unwrap(),
        Some(first)
    );
    let old = model.selected_job.clone();
    apply_key(
        &mut model,
        KeyEvent::new(KeyCode::Down, KeyModifiers::CONTROL),
    );
    if old == model.jobs.first().map(|job| job.id.clone()) {
        assert_ne!(old, model.selected_job);
    }
    model.selected_job = Some(second.clone());
    run_repl_line(&app, &mut model, "auth status")
        .await
        .unwrap();
    assert!(model
        .events
        .iter()
        .any(|event| event.starts_with("Account-opening URL:")));
    save_session(&app, &model).unwrap();
    let mut reopened = Model::new();
    load_session(&app, &mut reopened).unwrap();
    assert_eq!(reopened.selected_job, Some(second));
}
