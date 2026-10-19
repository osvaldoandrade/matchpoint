use super::tests::{key, screen};
use super::*;

#[test]
fn advanced_prompt_labels_use_english_on_home_and_jobs() {
    for home in [true, false] {
        for input in ["/", "/d", "help"] {
            for (w, h) in [(40, 18), (80, 24), (100, 30), (120, 40)] {
                let mut model = Model::new();
                model.home = home;
                model.command_mode = true;
                model.repl = input.into();
                let rendered = screen(&model, w, h);
                assert!(
                    rendered.contains("COMMAND · Enter submit"),
                    "{home} {input} {w}x{h}"
                );
                assert!(!rendered.contains("COMANDO"));
                assert!(!rendered.contains("executar"));
            }
        }
    }
}

#[test]
fn slash_menu_filters_completes_without_execution_and_scrolls_selection() {
    let mut model = Model::new();
    model.history.push("doctor".into());
    apply_key(&mut model, key(KeyCode::Char('/')));
    let rendered = screen(&model, 120, 24);
    assert!(rendered.contains("/help"));
    assert!(rendered.contains("Help and shortcuts"));
    apply_key(&mut model, key(KeyCode::Up));
    assert!(screen(&model, 120, 24).contains("> /quit"));
    assert_eq!(model.repl, "/");
    assert!(model.history_at.is_none());
    apply_key(&mut model, key(KeyCode::Down));
    assert!(screen(&model, 120, 24).contains("> /help"));
    apply_key(&mut model, key(KeyCode::Down));
    assert!(apply_key(&mut model, key(KeyCode::Enter)).is_none());
    assert_eq!(model.repl, "/panel");
    assert_eq!(model.panel, 0);
    assert_eq!(model.history, ["doctor"]);
    assert_eq!(
        apply_key(&mut model, key(KeyCode::Enter)).as_deref(),
        Some("/panel")
    );
    model.repl = "/pa".into();
    assert!(screen(&model, 120, 24).contains("/pause"));
    assert!(!commands::suggestions(&model)
        .unwrap()
        .iter()
        .any(|cmd| cmd.name == "/import"));
    apply_key(
        &mut model,
        KeyEvent::new(KeyCode::BackTab, KeyModifiers::SHIFT),
    );
    apply_key(&mut model, key(KeyCode::Tab));
    assert_eq!(model.repl, "/pause");
}

#[test]
fn dismissal_preserves_input_then_history_and_editing_reopen_suggestions() {
    let mut model = Model::new();
    model.history.push("/doctor".into());
    model.repl = "/d".into();
    apply_key(&mut model, key(KeyCode::Esc));
    assert_eq!(model.repl, "/d");
    assert!(commands::suggestions(&model).is_none());
    apply_key(&mut model, key(KeyCode::Backspace));
    assert!(commands::suggestions(&model).is_some());
    apply_key(&mut model, key(KeyCode::Esc));
    apply_key(&mut model, key(KeyCode::Up));
    assert_eq!(model.repl, "/doctor");
    apply_key(&mut model, key(KeyCode::Down));
    assert!(model.repl.is_empty());
    apply_key(&mut model, key(KeyCode::Char('/')));
    apply_key(&mut model, key(KeyCode::Esc));
    apply_key(&mut model, key(KeyCode::Esc));
    assert!(model.repl.is_empty());
    for ch in "/missing".chars() {
        apply_key(&mut model, key(KeyCode::Char(ch)));
    }
    assert!(screen(&model, 80, 24).contains("No commands"));
    for code in [KeyCode::Up, KeyCode::Down, KeyCode::Tab, KeyCode::BackTab] {
        apply_key(&mut model, key(code));
        assert_eq!(model.repl, "/missing");
    }
    assert_eq!(
        apply_key(&mut model, key(KeyCode::Enter)).as_deref(),
        Some("/missing")
    );
}

#[test]
fn slash_aliases_delegate_and_job_actions_require_selection() {
    let mut model = Model::new();
    for (alias, expected) in [
        ("/help", "help"),
        ("/api", "configure"),
        ("/jobs", "job list"),
        ("/doctor", "doctor"),
        ("/quit", "quit"),
        ("/job list", "job list"),
        ("/auth status", "auth status"),
        ("job list", "job list"),
    ] {
        assert_eq!(
            commands::resolve(&mut model, alias).unwrap().as_deref(),
            Some(expected),
            "{alias}"
        );
    }
    for verb in ["status", "run", "pause", "resume", "merge"] {
        let alias = format!("/{verb}");
        assert!(commands::resolve(&mut model, &alias).is_err(), "{alias}");
        model.selected_job = Some("selected-job".into());
        let result = commands::resolve(&mut model, &alias).unwrap();
        if verb == "merge" {
            assert!(result.is_none());
            assert_eq!(model.repl, "job merge selected-job --format parquet");
        } else {
            assert_eq!(result, Some(format!("job {verb} selected-job")));
        }
        model.selected_job = None;
    }
    assert_eq!(
        commands::resolve(&mut model, "/import").unwrap(),
        Some("import-form".into())
    );
    assert!(commands::resolve(&mut model, "/panel").unwrap().is_none());
    assert_eq!(model.panel, 1);
}

#[test]
fn ctrl_shortcuts_match_legacy_actions_and_do_not_insert_text() {
    for (ch, legacy) in [('g', 1), ('p', 2), ('o', 3), ('n', 4), ('r', 5), ('x', 6)] {
        let mut model = Model::new();
        let mut previous = Model::new();
        model.selected_job = Some("job-id".into());
        previous.selected_job = model.selected_job.clone();
        let result = apply_key(
            &mut model,
            KeyEvent::new(KeyCode::Char(ch), KeyModifiers::CONTROL),
        );
        assert_eq!(
            result,
            apply_key(&mut previous, key(KeyCode::F(legacy))),
            "Ctrl+{ch}"
        );
        assert_eq!(
            (model.help, model.panel, &model.repl),
            (previous.help, previous.panel, &previous.repl)
        );
    }
    let mut model = Model::new();
    assert!(apply_key(
        &mut model,
        KeyEvent::new(KeyCode::Char('r'), KeyModifiers::CONTROL)
    )
    .is_none());
    assert!(model.message.contains("N: create"));
    for modifiers in [KeyModifiers::ALT, KeyModifiers::CONTROL | KeyModifiers::ALT] {
        assert!(apply_key(&mut model, KeyEvent::new(KeyCode::Char('o'), modifiers)).is_none());
        assert!(model.repl.is_empty());
    }
    model.repl = "/api".into();
    let mut release = KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE);
    release.kind = KeyEventKind::Release;
    assert!(apply_key(&mut model, release).is_none());
    assert_eq!(model.repl, "/api");
    apply_key(
        &mut model,
        KeyEvent::new(KeyCode::Char('u'), KeyModifiers::CONTROL),
    );
    assert!(model.repl.is_empty());
    assert_eq!(
        apply_key(
            &mut model,
            KeyEvent::new(KeyCode::Char('c'), KeyModifiers::CONTROL)
        )
        .as_deref(),
        Some("quit")
    );
}

#[test]
fn command_menu_handles_terminal_boundaries_and_hides_secrets() {
    let mut model = Model::new();
    for (w, h) in [(0, 0), (1, 1), (20, 5), (40, 10), (80, 24), (120, 40)] {
        for input in [
            "/",
            "/run",
            "/missing",
            "/auth configure --client-secret=test-only-secret",
        ] {
            model.repl = input.into();
            let rendered = screen(&model, w, h);
            assert!(!rendered.contains("test-only-secret"));
            if w >= 20 && h >= 5 && input == "/" {
                assert!(rendered.contains("/help"), "{w}x{h}");
            }
        }
    }
    model.repl = "/auth configure --client-secret=test-only-secret".into();
    assert!(commands::suggestions(&model).is_none());
    apply_key(&mut model, key(KeyCode::Enter));
    assert!(model.history.is_empty());
}

#[test]
fn input_is_bounded_in_bytes_and_history_keeps_only_200_safe_commands() {
    for (len, ch, expected) in [(8191, 'é', 8191), (8190, 'é', 8192), (8191, 'a', 8192)] {
        let mut model = Model::new();
        model.repl = "x".repeat(len);
        apply_key(&mut model, key(KeyCode::Char(ch)));
        assert_eq!(model.repl.len(), expected);
    }
    let mut model = Model::new();
    model.command_mode = true;
    assert!(apply_key(&mut model, key(KeyCode::Enter)).is_none());
    model.history = (0..200).map(|i| format!("job status job-{i}")).collect();
    model.repl = "/doctor".into();
    assert_eq!(
        apply_key(&mut model, key(KeyCode::Enter)).as_deref(),
        Some("/doctor")
    );
    assert_eq!(model.history.len(), 200);
    assert_eq!(model.history[0], "job status job-1");
    assert_eq!(model.history.last().unwrap(), "/doctor");
    apply_key(
        &mut model,
        KeyEvent::new(KeyCode::Char('b'), KeyModifiers::CONTROL),
    );
    assert!(model.selected_job.is_none());
}

#[tokio::test]
async fn slash_commands_reach_existing_executor_and_form_keeps_keyboard_focus() {
    use std::sync::Arc;
    let dir = tempfile::tempdir().unwrap();
    let app = App::open(
        dir.path().join("data"),
        Arc::new(crate::secrets::MemorySecrets::new()),
        Arc::new(crate::timeutil::SystemClock),
    )
    .unwrap();
    let mut model = Model::new();
    for input in [
        "/missing",
        "/run",
        "/auth configure --client-secret=test-only-secret",
    ] {
        assert!(
            run_repl_line(&app, &mut model, input).await.is_err(),
            "{input}"
        );
    }
    for input in ["/help", "/panel", "/import", "/jobs", "/auth status"] {
        assert!(run_repl_line(&app, &mut model, input)
            .await
            .unwrap()
            .is_none());
    }
    model.repl = "/".into();
    run_repl_line(&app, &mut model, "/api").await.unwrap();
    assert!(model.settings.is_some());
    apply_key(&mut model, key(KeyCode::Tab));
    apply_key(&mut model, key(KeyCode::Char('/')));
    apply_key(
        &mut model,
        KeyEvent::new(KeyCode::Char('n'), KeyModifiers::CONTROL),
    );
    assert_eq!(model.settings.as_ref().unwrap().api.fields()[0].1, "/");
    apply_key(
        &mut model,
        KeyEvent::new(KeyCode::Char('u'), KeyModifiers::CONTROL),
    );
    assert!(model.settings.as_ref().unwrap().api.fields()[0]
        .1
        .is_empty());
    assert!(model.repl.is_empty());
    apply_key(
        &mut model,
        KeyEvent::new(KeyCode::Char('f'), KeyModifiers::CONTROL),
    );
    assert_eq!(model.settings.as_ref().unwrap().focus, Some(0));
    apply_key(&mut model, key(KeyCode::Tab));
    assert_eq!(model.settings.as_ref().unwrap().focus, Some(1));
    apply_key(&mut model, key(KeyCode::Esc));
    assert!(model.settings.is_none());
    model.busy = true;
    run_repl_line(&app, &mut model, "/api").await.unwrap();
    assert!(run_repl_line(&app, &mut model, "save-settings")
        .await
        .is_err());
    apply_key(&mut model, key(KeyCode::Esc));
    let input = dir.path().join("input.csv");
    std::fs::write(&input, "52998224725\n").unwrap();
    for _ in 0..2 {
        app.create_job(&input, crate::jobcfg::default_tuning(), None)
            .unwrap();
    }
    refresh(&app, &mut model).await.unwrap();
    let first = model.jobs[0].id.clone();
    let last = model.jobs[1].id.clone();
    model.repl = "/".into();
    for (ch, expected) in [
        ('b', &first),
        ('f', &last),
        ('f', &last),
        ('b', &first),
        ('b', &first),
    ] {
        apply_key(
            &mut model,
            KeyEvent::new(KeyCode::Char(ch), KeyModifiers::CONTROL),
        );
        assert_eq!(model.selected_job.as_ref(), Some(expected), "Ctrl+{ch}");
        assert_eq!(model.repl, "/");
    }
    assert!(run_repl_line(&app, &mut model, "/run").await.is_err());
    model.busy = false;
    assert_eq!(
        run_repl_line(&app, &mut model, "/run").await.unwrap(),
        Some(first)
    );
    assert!(!app.paths.config_file().exists());
    assert!(model
        .events
        .iter()
        .all(|event| !event.contains("test-only-secret")));
}
