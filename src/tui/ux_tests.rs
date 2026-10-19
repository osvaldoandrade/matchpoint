use super::tests::{key, screen};
use super::*;
use crate::secrets::MemorySecrets;
use crate::timeutil::SystemClock;

#[test]
fn home_is_a_menu_and_shifted_keys_and_backward_navigation_are_supported() {
    let mut model = Model::new();
    assert!(model.home);
    let rendered = screen(&model, 80, 24);
    assert!(rendered.contains("[J] JOBS"));
    assert!(!rendered.contains("PRIMEIRO ACESSO"));
    assert!(!rendered.contains("COMANDO"));
    apply_key(
        &mut model,
        KeyEvent::new(KeyCode::BackTab, KeyModifiers::SHIFT),
    );
    assert_eq!(model.home_selection, 2);
    assert_eq!(
        apply_key(&mut model, key(KeyCode::Enter)).as_deref(),
        Some("settings-view")
    );
    assert_eq!(
        apply_key(
            &mut model,
            KeyEvent::new(KeyCode::Char('Q'), KeyModifiers::SHIFT)
        )
        .as_deref(),
        Some("quit")
    );
    apply_key(&mut model, key(KeyCode::Char('/')));
    assert!(model.command_mode);
}

#[test]
fn new_job_is_first_and_home_shortcuts_match_the_visible_order() {
    let actions = ["import-form", "jobs-view", "settings-view"];
    for (width, height) in [(120, 40), (100, 30), (80, 24), (40, 18)] {
        let mut model = Model::new();
        let view = screen(&model, width, height);
        assert!(view.find("[N] NEW JOB").unwrap() < view.find("[J] JOBS").unwrap());
        for (index, action) in actions.iter().enumerate() {
            assert_eq!(
                apply_key(&mut model, key(KeyCode::Enter)).as_deref(),
                Some(*action)
            );
            assert_eq!(
                apply_key(
                    &mut model,
                    key(KeyCode::Char(char::from(b'1' + index as u8)))
                )
                .as_deref(),
                Some(*action)
            );
            apply_key(&mut model, key(KeyCode::Right));
        }
        assert!(model.home && model.jobs.is_empty() && model.import_form.is_none());
        for (ch, action) in [('N', actions[0]), ('J', actions[1]), ('S', actions[2])] {
            assert_eq!(
                apply_key(
                    &mut model,
                    KeyEvent::new(KeyCode::Char(ch), KeyModifiers::SHIFT)
                )
                .as_deref(),
                Some(action)
            );
        }
    }
}

#[test]
fn partner_home_menu_descriptions_follow_keys_and_show_real_workspace_counts() {
    let mut model = Model::new();
    for (width, height) in [(120, 40), (100, 30), (80, 24), (40, 18)] {
        model.home_selection = 0;
        for (index, description) in copy::HOME_DESCRIPTIONS.iter().enumerate() {
            let view = screen(&model, width, height);
            assert!(view.contains(copy::HOME_HEADLINE));
            assert!(view.contains(copy::HOME_SUBHEADLINE));
            assert!(view.contains(description), "{width}x{height}: {index}");
            assert!(!view.contains("COMMAND CENTER"));
            assert!(view.contains("Q quit"));
            apply_key(&mut model, key(KeyCode::Right));
        }
        assert_eq!(model.home_selection, 0);
        apply_key(&mut model, key(KeyCode::Left));
        assert_eq!(model.home_selection, 2);
        assert_eq!(
            apply_key(&mut model, key(KeyCode::Enter)).as_deref(),
            Some("settings-view")
        );
    }
    model.jobs.push(crate::engine::JobStatus {
        id: "synthetic-job".into(),
        state: "PREPARED".into(),
        desired: "pause".into(),
        counts: crate::domain::Counts {
            pending: 42,
            succeeded: 8,
            unknown: 3,
            ..Default::default()
        },
        batches: 7,
    });
    model.selected_job = Some("synthetic-job".into());
    let view = screen(&model, 100, 30);
    for (label, value) in [
        ("jobs", "1"),
        ("batches", "7"),
        ("Processed", "/53"),
        ("Queue", "42"),
        ("Unknown", "3"),
    ] {
        assert!(view
            .lines()
            .any(|line| line.contains(label) && line.contains(value)));
    }
    assert!(view.contains("syntheti") && view.contains("Ready"));
    assert!(view.contains("API setup needed"));
    model.api_ready = true;
    assert!(screen(&model, 100, 30).contains("API setup enabled"));
}

#[test]
fn splash_requires_explicit_continue_preserves_selection_and_ignores_paste() {
    let mut model = Model::new();
    model.splash = true;
    model.selected_job = Some("saved-job".into());
    for code in [
        KeyCode::Char('n'),
        KeyCode::Char('/'),
        KeyCode::Char('s'),
        KeyCode::F(4),
    ] {
        assert!(apply_key(&mut model, key(code)).is_none());
        assert!(model.splash && model.import_form.is_none() && model.settings.is_none());
    }
    apply_paste(&mut model, "synthetic-secret\n/quit");
    assert!(model.repl.is_empty() && model.history.is_empty() && model.pending_work.is_none());
    assert!(apply_key(&mut model, key(KeyCode::Enter)).is_none());
    assert!(!model.splash && model.home);
    assert_eq!(model.selected_job.as_deref(), Some("saved-job"));
    for event in [
        key(KeyCode::Esc),
        key(KeyCode::Char('q')),
        KeyEvent::new(KeyCode::Char('c'), KeyModifiers::CONTROL),
    ] {
        model.splash = true;
        assert_eq!(apply_key(&mut model, event).as_deref(), Some("quit"));
    }
}

#[tokio::test]
async fn direct_error_preserves_menu_pause_and_notifies_the_actual_cause() {
    let (tmp, app) = app();
    let path = tmp.path().join("in.csv");
    std::fs::write(&path, "52998224725\n").unwrap();
    let id = app
        .create_job(&path, crate::jobcfg::default_tuning(), None)
        .unwrap();
    let mut model = Model::new();
    model.home = false;
    model.selected_job = Some(id.clone());
    model.active_job = Some(id.clone());
    model.busy = true;
    refresh(&app, &mut model).await.unwrap();
    let action = apply_key(&mut model, key(KeyCode::Char('t'))).unwrap();
    let err = run_repl_line(&app, &mut model, &action).await.unwrap_err();
    command_error(&mut model, &err, &action, false);
    assert!(model.repl.is_empty());
    assert!(!model.command_mode);
    let rendered = screen(&model, 40, 18);
    assert!(rendered.contains("Threads locked"));
    assert!(rendered.contains("P: pause"));
    let pause = apply_key(&mut model, key(KeyCode::Char('p'))).unwrap();
    assert_eq!(
        commands::resolve(&mut model, &pause).unwrap().unwrap(),
        format!("job pause {id}")
    );
}

#[tokio::test]
async fn run_result_identifies_its_origin_after_selection_changes() {
    let (tmp, app) = app();
    let path = tmp.path().join("two.csv");
    std::fs::write(&path, "52998224725\n").unwrap();
    let active = app
        .create_job(&path, crate::jobcfg::default_tuning(), None)
        .unwrap();
    let selected = app
        .create_job(&path, crate::jobcfg::default_tuning(), None)
        .unwrap();
    let mut model = Model::new();
    model.home = false;
    model.active_job = Some(active.clone());
    model.selected_job = Some(selected.clone());
    model.busy = true;
    finish_run(
        &mut model,
        Ok(RunReport {
            state: "PAUSED".into(),
            paused: true,
            interrupted: false,
        }),
    );
    refresh(&app, &mut model).await.unwrap();
    assert_eq!(model.selected_job.as_ref(), Some(&selected));
    assert!(model.message.contains(&active[..8]));
    assert!(model.message.contains("Paused"));
    assert!(!model.message.contains("/status"));
    assert!(!model.busy);
    assert!(model.active_job.is_none());
    model.active_job = Some(active.clone());
    finish_run(&mut model, Err(Error::AuthPaused));
    refresh(&app, &mut model).await.unwrap();
    assert!(model.error.as_ref().unwrap().title.contains(&active[..8]));
}

#[test]
fn unknown_menu_keys_and_pastes_do_not_enter_or_execute_commands() {
    let mut model = Model::new();
    let events = model.events.len();
    for home in [true, false] {
        model.home = home;
        assert!(apply_key(&mut model, key(KeyCode::Char('z'))).is_none());
        apply_paste(&mut model, "q\nr\n/quit");
        assert!(model.repl.is_empty());
        assert!(!model.command_mode);
    }
    model.import_form = Some(forms::ImportForm::new(8));
    apply_paste(&mut model, "/tmp/clientes de teste.csv\n");
    assert_eq!(
        model.import_form.as_ref().unwrap().path,
        "/tmp/clientes de teste.csv"
    );
    assert!(model.pending_work.is_none());
    assert!(model.history.is_empty());
    assert_eq!(model.events.len(), events);
}

#[tokio::test]
async fn editing_a_rejected_field_removes_stale_error_without_saving() {
    let (_tmp, app) = app();
    let mut model = Model::new();
    run_repl_line(&app, &mut model, "configure").await.unwrap();
    let err = run_repl_line(&app, &mut model, "save-api")
        .await
        .unwrap_err();
    model.show_error(&err);
    assert!(model.error.is_some());
    model.settings.as_mut().unwrap().tab = 1;
    model.settings.as_mut().unwrap().focus = Some(0);
    model.settings.as_mut().unwrap().api.focus = 3;
    apply_paste(&mut model, "synthetic\nsecret");
    assert!(model.error.is_none());
    assert!(!app.paths.config_file().exists());
    assert!(!screen(&model, 40, 18).contains("synthetic"));
    model.show_error(&Error::InsecureUrl);
    apply_key(&mut model, key(KeyCode::Char('x')));
    assert!(model.error.is_none());
}

#[tokio::test]
async fn diagnosis_is_in_english_and_keeps_skipped_checks_explicit() {
    let (_tmp, app) = app();
    let mut model = Model::new();
    run_repl_line(&app, &mut model, "doctor").await.unwrap();
    let text = model.events.back().unwrap();
    assert!(text.contains("LOCAL DIAGNOSTICS"));
    assert!(text.contains("client ID"));
    assert!(text.contains("Keychain: not tested"));
    assert!(!text.contains("keychain_probe"));
    assert!(!text.contains("local disk"));
    assert_eq!(model.panel, 3);
    assert!(!model.home);
}

#[tokio::test]
async fn failed_async_import_preserves_path_threads_and_validation_for_retry() {
    let (tmp, app) = app();
    let path = tmp.path().join("broken input.csv");
    std::fs::write(&path, "52998224725\n\"unterminated\n").unwrap();
    let mut model = Model::new();
    run_repl_line(&app, &mut model, "import-form")
        .await
        .unwrap();
    let form = model.import_form.as_mut().unwrap();
    form.path = path.to_str().unwrap().into();
    form.threads = "16".into();
    form.validation = 0;
    run_repl_line(&app, &mut model, "save-import")
        .await
        .unwrap();
    let err = perform_pending_work(&app, &mut model).await.unwrap_err();
    model.show_error(&err);
    run_repl_line(&app, &mut model, "import-form")
        .await
        .unwrap();
    let form = model.import_form.as_ref().unwrap();
    assert_eq!(form.path, path.to_str().unwrap());
    assert_eq!(form.threads, "16");
    assert_eq!(form.validation, 0);
    std::fs::write(&path, "52998224725\n").unwrap();
    run_repl_line(&app, &mut model, "save-import")
        .await
        .unwrap();
    perform_pending_work(&app, &mut model).await.unwrap();
    assert_eq!(model.selected_config.as_ref().unwrap().concurrency, 16);
    assert!(model.import_draft.is_none());
}

#[test]
fn compact_home_and_jobs_keep_complete_exit_and_checksum_shortcuts() {
    let mut model = Model::new();
    for (width, height) in [(120, 40), (100, 30), (80, 24), (40, 18)] {
        let home = screen(&model, width, height);
        assert!(home.contains("Parter Get Parter"));
        let footer = home.lines().find(|line| line.contains("Q quit")).unwrap();
        assert!(footer.contains("Enter") && footer.contains('/') && footer.contains("? help"));
        if width >= 80 {
            assert!(footer.contains("←→ select") && footer.contains("/ commands"));
        }
        model.splash = true;
        for motion in [true, false] {
            model.motion = motion;
            let splash = screen(&model, width, height);
            assert!(splash.contains("Parter Get Parter") && splash.contains("Esc quit"));
            assert!(!splash.contains("Space") && !splash.contains("pause animation"));
        }
        model.splash = false;
    }
    model.home = false;
    let jobs = screen(&model, 40, 18);
    assert!(jobs.contains("Q quit"));
    model.panel = 1;
    assert!(screen(&model, 40, 18).contains("K checksum"));
}

#[test]
fn home_and_compact_forms_show_errors_without_hiding_recovery() {
    let mut model = Model::new();
    model.show_error(&Error::ThreadsBusy);
    let home = screen(&model, 40, 18);
    assert!(home.contains("Threads locked"));
    assert!(home.contains("P: pause"));
    model.import_form = Some(forms::ImportForm::new(8));
    model.show_error(&Error::Io(std::io::Error::from(
        std::io::ErrorKind::NotFound,
    )));
    let form = screen(&model, 40, 18);
    assert!(form.contains("File not found"));
    assert!(form.contains("Fix the path; Enter creates."));
    assert!(form.contains("Enter: create"));
}

#[tokio::test]
async fn appearance_failed_launch_retains_choices_for_retry_and_saves_motion() {
    let (_tmp, app) = app();
    let mut model = Model::new();
    run_repl_line(&app, &mut model, "appearance-form")
        .await
        .unwrap();
    let settings = model.settings.as_mut().unwrap();
    settings.focus = Some(4);
    let form = &mut settings.appearance;
    form.font_size = 20;
    form.speed = 0;
    let action = apply_key(&mut model, key(KeyCode::Enter)).unwrap();
    assert_eq!(action, "settings-ghostty");
    assert!(run_repl_line(&app, &mut model, &action).await.is_err());
    assert_eq!(model.settings.as_ref().unwrap().appearance.font_size, 20);
    let mut reopened = Model::new();
    load_session(&app, &mut reopened).unwrap();
    assert_eq!(reopened.font_size, 20);
    assert!(!reopened.motion);
}

#[tokio::test]
async fn batch_paging_reaches_every_batch_on_each_supported_viewport() {
    let (tmp, app) = app();
    let path = tmp.path().join("many.csv");
    std::fs::write(&path, "52998224725\n".repeat(25)).unwrap();
    let mut cfg = crate::jobcfg::default_tuning();
    cfg.batch_size = 1;
    let id = app.create_job(&path, cfg, None).unwrap();
    for (w, h) in [(120, 40), (100, 30), (80, 24), (40, 18)] {
        let mut model = Model::new();
        model.home = false;
        model.panel = 2;
        model.viewport = (w, h);
        model.selected_job = Some(id.clone());
        let mut seen = std::collections::BTreeSet::new();
        for _ in 0..40 {
            refresh(&app, &mut model).await.unwrap();
            let rendered = screen(&model, w, h);
            for n in 1..=25 {
                if rendered.contains(&format!(" {n:<4} ")) {
                    seen.insert(n);
                }
            }
            if model.batch_offset >= 24 {
                break;
            }
            apply_key(&mut model, key(KeyCode::PageDown));
        }
        assert_eq!(seen.len(), 25, "{w}x{h}: {seen:?}");
    }
}

#[tokio::test]
async fn checksum_feedback_is_cleared_when_selecting_another_job() {
    let (tmp, app) = app();
    let path = tmp.path().join("input.csv");
    std::fs::write(&path, "52998224725\n").unwrap();
    let first = app
        .create_job(&path, crate::jobcfg::default_tuning(), None)
        .unwrap();
    let second = app
        .create_job(&path, crate::jobcfg::default_tuning(), None)
        .unwrap();
    let mut model = Model::new();
    model.selected_job = Some(first);
    let selected = model.selected_job.clone().unwrap();
    std::fs::write(app.paths.job_dir(&selected).join("input.csv"), "changed\n").unwrap();
    run_repl_line(&app, &mut model, "/checksum").await.unwrap();
    perform_pending_work(&app, &mut model).await.unwrap();
    assert!(model.message.contains("MISMATCH"));
    model.selected_job = Some(second);
    refresh(&app, &mut model).await.unwrap();
    assert!(!model.message.contains("MISMATCH"));
    assert!(model.checksum.is_none());
}

#[test]
fn splash_and_titles_animate_reduced_motion_is_fixed_and_home_charts_stay_static() {
    use ratatui::{backend::TestBackend, Terminal};
    let mut model = Model::new();
    model.splash = true;
    let mut terminal = Terminal::new(TestBackend::new(100, 30)).unwrap();
    terminal.draw(|f| render(f, &model)).unwrap();
    let first = terminal.backend().buffer().clone();
    model.animation = 28;
    terminal.draw(|f| render(f, &model)).unwrap();
    assert_ne!(first, *terminal.backend().buffer());
    model.motion = false;
    terminal.draw(|f| render(f, &model)).unwrap();
    let fixed = terminal.backend().buffer().clone();
    model.animation = 70;
    terminal.draw(|f| render(f, &model)).unwrap();
    assert_eq!(fixed, *terminal.backend().buffer());
    model.splash = false;
    model.motion = true;
    model.animation = 0;
    terminal.draw(|f| render(f, &model)).unwrap();
    let home = terminal.backend().buffer().clone();
    model.animation = 28;
    terminal.draw(|f| render(f, &model)).unwrap();
    assert_ne!(home, *terminal.backend().buffer());
    for y in 3..30 {
        for x in 0..100 {
            assert_eq!(
                home[(x, y)],
                terminal.backend().buffer()[(x, y)],
                "dashboard or menu changed at {x},{y}"
            );
        }
    }
    model.motion = false;
    apply_key(&mut model, key(KeyCode::Right));
    terminal.draw(|f| render(f, &model)).unwrap();
    let fixed = terminal.backend().buffer().clone();
    model.animation = 180;
    terminal.draw(|f| render(f, &model)).unwrap();
    assert_eq!(fixed, *terminal.backend().buffer());
}
use std::sync::Arc;

fn app() -> (tempfile::TempDir, App) {
    let tmp = tempfile::tempdir().unwrap();
    let app = App::open(
        tmp.path().join("data"),
        Arc::new(MemorySecrets::new()),
        Arc::new(SystemClock),
    )
    .unwrap();
    (tmp, app)
}

#[tokio::test]
async fn import_form_handles_spaces_errors_and_selects_created_job() {
    let (tmp, app) = app();
    let path = tmp.path().join("clientes de teste.csv");
    std::fs::write(&path, "52998224725\n39053344705\n").unwrap();
    let mut model = Model::new();
    run_repl_line(&app, &mut model, "/import").await.unwrap();
    assert!(run_repl_line(&app, &mut model, "save-import")
        .await
        .is_err());
    assert!(model.import_form.is_some());
    for ch in path.to_str().unwrap().chars() {
        apply_key(&mut model, key(KeyCode::Char(ch)));
    }
    let save = apply_key(&mut model, key(KeyCode::Enter)).unwrap();
    run_repl_line(&app, &mut model, &save).await.unwrap();
    perform_pending_work(&app, &mut model).await.unwrap();
    let id = model.selected_job.as_ref().unwrap();
    assert_eq!(app.status(id).unwrap().counts.total(), 2);
    assert_eq!(
        app.job_config(id).unwrap().input_name,
        "clientes de teste.csv"
    );
    assert!(model.events.iter().any(|line| line.contains("Imported:")));
}

#[tokio::test]
async fn thread_form_validates_cancels_persists_and_rejects_running_job() {
    let (tmp, app) = app();
    let path = tmp.path().join("in.csv");
    std::fs::write(&path, "52998224725\n").unwrap();
    let id = app
        .create_job(&path, crate::jobcfg::default_tuning(), None)
        .unwrap();
    let mut model = Model::new();
    model.selected_job = Some(id.clone());
    refresh(&app, &mut model).await.unwrap();
    run_repl_line(&app, &mut model, "/threads").await.unwrap();
    model.thread_form.as_mut().unwrap().value = "0".into();
    assert!(run_repl_line(&app, &mut model, "save-threads")
        .await
        .is_err());
    assert_eq!(app.job_config(&id).unwrap().concurrency, 8);
    model.thread_form.as_mut().unwrap().value = "16".into();
    apply_key(&mut model, key(KeyCode::Esc));
    assert_eq!(app.job_config(&id).unwrap().concurrency, 8);
    run_repl_line(&app, &mut model, "/threads").await.unwrap();
    model.thread_form.as_mut().unwrap().value = "16".into();
    run_repl_line(&app, &mut model, "save-threads")
        .await
        .unwrap();
    assert_eq!(app.job_config(&id).unwrap().concurrency, 16);
    assert_eq!(app.job_config(&id).unwrap().rps, 10.0);
    run_repl_line(&app, &mut model, "/threads-default")
        .await
        .unwrap();
    model.settings.as_mut().unwrap().threads = "12".into();
    run_repl_line(&app, &mut model, "save-threads")
        .await
        .unwrap();
    assert_eq!(app.thread_defaults().unwrap().threads, 12);
    let reopened = App::open(
        app.paths.root.clone(),
        Arc::new(MemorySecrets::new()),
        Arc::new(SystemClock),
    )
    .unwrap();
    assert_eq!(reopened.thread_defaults().unwrap().threads, 12);
    let db = crate::store::Db::open(&app.paths.job_dir(&id).join("job.sqlite")).unwrap();
    db.set_state(crate::domain::JobState::Running).unwrap();
    assert!(run_repl_line(&app, &mut model, "/threads").await.is_err());
    assert!(app.configure_threads(&id, 20).is_err());
}

#[tokio::test]
async fn checksum_reports_full_hash_detects_tamper_and_missing_export() {
    let (tmp, app) = app();
    let path = tmp.path().join("invalid.csv");
    std::fs::write(&path, "invalid\n").unwrap();
    let id = app
        .create_job(&path, crate::jobcfg::default_tuning(), None)
        .unwrap();
    let merged = app
        .merge(&id, crate::engine::OutFormat::Csv, false, false)
        .unwrap();
    let report = app.verify_checksum(&id).unwrap();
    assert!(report.matches());
    assert_eq!(report.artifacts.len(), 2);
    assert!(report
        .artifacts
        .iter()
        .all(|a| a.expected.len() == 64 && a.actual.len() == 64));
    assert_eq!(report.artifacts[1].expected, merged.sha256);
    std::fs::write(app.paths.job_dir(&id).join("input.csv"), "changed\n").unwrap();
    assert!(!app.verify_checksum(&id).unwrap().matches());
    std::fs::remove_file(app.paths.job_dir(&id).join("exports/gen-0001/merged.csv")).unwrap();
    let report = app.verify_checksum(&id).unwrap();
    assert_eq!(report.artifacts[1].actual, "FILE MISSING");
    assert!(!report.matches());
}

#[test]
fn progress_is_truthful_animated_and_visible_at_all_supported_sizes() {
    let mut model = Model::new();
    model.home = false;
    let id = "01234567-0123-4567-8901-234567890123".to_string();
    model.selected_job = Some(id.clone());
    model.jobs.push(JobStatus {
        id,
        state: "RUNNING".into(),
        desired: "run".into(),
        counts: crate::domain::Counts {
            pending: 8,
            succeeded: 2,
            ..Default::default()
        },
        batches: 1,
    });
    for (w, h) in [(120, 40), (100, 30), (80, 24), (40, 18)] {
        let first = screen(&model, w, h);
        assert!(first.contains("20%"), "{w}x{h}");
        model.animation = 2;
        let second = screen(&model, w, h);
        assert!(second.contains("20%"));
        assert!(second.contains('▓'), "{w}x{h}");
        model.animation = 0;
    }
    model.motion = false;
    let first = screen(&model, 80, 24);
    model.animation = 4;
    assert_eq!(first, screen(&model, 80, 24));
}

#[test]
fn pause_targets_active_job_even_after_selection_changes_and_unicode_editing_works() {
    let mut model = Model::new();
    model.active_job = Some("active".into());
    model.selected_job = Some("selected".into());
    assert_eq!(
        apply_key(&mut model, key(KeyCode::F(6))).unwrap(),
        "job pause active"
    );
    assert_eq!(
        commands::resolve(&mut model, "/pause").unwrap().unwrap(),
        "job pause active"
    );
    model.repl = "aéz".into();
    apply_key(&mut model, key(KeyCode::Left));
    apply_key(&mut model, key(KeyCode::Backspace));
    assert_eq!(model.repl, "az");
    apply_key(&mut model, key(KeyCode::Char('ê')));
    assert_eq!(model.repl, "aêz");
    apply_key(&mut model, key(KeyCode::Delete));
    assert_eq!(model.repl, "aê");
}
