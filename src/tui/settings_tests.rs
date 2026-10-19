use super::{
    tests::{key, screen},
    *,
};
use crate::{
    engine::ProcessingDefaults,
    error::Secret,
    secrets::{MemorySecrets, SecretStore, ACCOUNT_REFRESH},
    timeutil::SystemClock,
};
use std::sync::Arc;

fn setup() -> (tempfile::TempDir, App, Arc<MemorySecrets>) {
    let dir = tempfile::tempdir().unwrap();
    let secrets = Arc::new(MemorySecrets::new());
    let app = App::open(
        dir.path().join("data"),
        secrets.clone(),
        Arc::new(SystemClock),
    )
    .unwrap();
    (dir, app, secrets)
}
#[tokio::test]
async fn settings_tabs_keep_drafts_and_save_only_the_active_group() {
    let (_dir, app, _) = setup();
    let mut model = Model::new();
    run_repl_line(&app, &mut model, "settings-view")
        .await
        .unwrap();
    apply_key(&mut model, key(KeyCode::Tab));
    apply_paste(&mut model, "unsaved-client");
    for _ in 0..2 {
        apply_key(&mut model, key(KeyCode::F(2)));
    }
    apply_key(&mut model, key(KeyCode::Tab));
    apply_key(
        &mut model,
        KeyEvent::new(KeyCode::Char('u'), KeyModifiers::CONTROL),
    );
    apply_paste(&mut model, "24");
    run_repl_line(&app, &mut model, "save-settings")
        .await
        .unwrap();
    assert_eq!(app.processing_defaults().unwrap().threads, 24);
    assert!(!app.paths.config_file().exists());
    assert_eq!(
        model.settings.as_ref().unwrap().api.settings.client_id,
        "unsaved-client"
    );
    assert!(model.settings.as_ref().unwrap().dirty[0]);
    let rendered = screen(&model, 40, 18);
    assert!(rendered.contains("API*"));
    assert!(rendered.contains("* unsaved"));
    apply_key(&mut model, key(KeyCode::F(2)));
    assert!(!screen(&model, 40, 18).contains("Processing defaults saved"));
    apply_key(&mut model, key(KeyCode::Esc));
    assert!(model.settings.is_none());
    assert!(model.message.contains("unsaved edits discarded"));
}
#[tokio::test]
async fn all_tabs_have_english_fields_and_reduced_motion_preferences_reopen() {
    let (_dir, app, _) = setup();
    let mut model = Model::new();
    run_repl_line(&app, &mut model, "settings-view")
        .await
        .unwrap();
    for tab in 0..4 {
        assert_eq!(model.settings.as_ref().unwrap().tab, tab);
        for (w, h) in [(40, 18), (80, 24), (100, 30), (120, 40)] {
            let text = screen(&model, w, h);
            assert!(text.contains("SETTINGS"));
            assert!(text.contains("Ctrl+S save"));
            assert!(text.contains(copy::SETTINGS_TABS[tab]));
        }
        if tab < 3 {
            apply_key(&mut model, key(KeyCode::Right));
        }
    }
    let form = &mut model.settings.as_mut().unwrap().appearance;
    form.scene = SceneKind::Tesseract;
    form.speed = 0;
    form.font_size = 22;
    run_repl_line(&app, &mut model, "save-settings")
        .await
        .unwrap();
    let mut reopened = Model::new();
    load_session(&app, &mut reopened).unwrap();
    assert_eq!(reopened.scene, SceneKind::Tesseract);
    assert!(!reopened.motion);
    assert_eq!(reopened.font_size, 22);
}
#[test]
fn processing_defaults_read_old_files_validate_boundaries_and_preserve_other_values() {
    let (_dir, app, _) = setup();
    std::fs::write(app.paths.root.join("processing.json"), r#"{"threads":17}"#).unwrap();
    assert_eq!(app.processing_defaults().unwrap().threads, 17);
    let defaults = ProcessingDefaults {
        threads: 17,
        rps: 3.5,
        batch_size: 500,
        validation: "none".into(),
    };
    app.set_processing_defaults(defaults.clone()).unwrap();
    app.set_default_threads(19).unwrap();
    let saved = app.processing_defaults().unwrap();
    assert_eq!(saved.rps, 3.5);
    assert_eq!(saved.batch_size, 500);
    assert_eq!(saved.validation, "none");
    for (threads, rps, batch, valid) in [
        (1, 0.1, 1, true),
        (256, 100000., 100000, true),
        (0, 10., 1000, false),
        (257, 10., 1000, false),
        (8, 0., 1000, false),
        (8, f64::NAN, 1000, false),
        (8, f64::INFINITY, 1000, false),
        (8, 100001., 1000, false),
        (8, 10., 0, false),
        (8, 10., 100001, false),
    ] {
        let value = ProcessingDefaults {
            threads,
            rps,
            batch_size: batch,
            ..defaults.clone()
        };
        assert_eq!(value.validate().is_ok(), valid);
    }
    assert!(ProcessingDefaults {
        validation: "invalid".into(),
        ..defaults
    }
    .validate()
    .is_err());
}
#[test]
fn metadata_save_keeps_credentials_and_each_changed_identity_component_requires_a_secret() {
    let (_dir, app, secrets) = setup();
    app.configure_api(
        crate::engine::AppConfig {
            client_id: "fixture".into(),
            process_url: "https://example.test/original-data".into(),
            token_url: "https://example.test/original-token".into(),
            ..Default::default()
        },
        Secret::new("synthetic-secret".into()).unwrap(),
        Some(Secret::new("synthetic-refresh".into()).unwrap()),
    )
    .unwrap();
    let mut profile = app.api_config().unwrap();
    profile.remote_enabled = true;
    app.configure_api_metadata(profile.clone()).unwrap();
    assert_eq!(
        secrets.get(ACCOUNT_REFRESH).unwrap().unwrap().expose(),
        "synthetic-refresh"
    );
    for field in 0..3 {
        let mut changed = profile.clone();
        match field {
            0 => changed.client_id = "different".into(),
            1 => changed.process_url = "https://example.test/process".into(),
            _ => changed.token_url = "https://example.test/token".into(),
        }
        assert!(app.configure_api_metadata(changed).is_err());
        assert_eq!(app.api_config().unwrap().client_id, "fixture");
    }
    std::fs::remove_file(app.paths.config_file()).unwrap();
    assert!(app.configure_api_metadata(profile).is_err());
}
#[tokio::test]
async fn invalid_processing_draft_stays_editable_and_import_inherits_saved_defaults() {
    let (dir, app, _) = setup();
    let mut model = Model::new();
    run_repl_line(&app, &mut model, "default-threads-form")
        .await
        .unwrap();
    model.settings.as_mut().unwrap().threads = "0".into();
    assert!(run_repl_line(&app, &mut model, "save-settings")
        .await
        .is_err());
    assert!(model.settings.is_some());
    assert_eq!(app.processing_defaults().unwrap().threads, 8);
    let settings = model.settings.as_mut().unwrap();
    settings.threads = "13".into();
    settings.rps = "4.5".into();
    settings.batch_size = "2".into();
    settings.validation = 0;
    run_repl_line(&app, &mut model, "save-settings")
        .await
        .unwrap();
    apply_key(&mut model, key(KeyCode::Esc));
    let path = dir.path().join("sample.csv");
    std::fs::write(&path, "52998224725\n").unwrap();
    run_repl_line(&app, &mut model, "import-form")
        .await
        .unwrap();
    apply_paste(&mut model, path.to_str().unwrap());
    run_repl_line(&app, &mut model, "save-import")
        .await
        .unwrap();
    perform_pending_work(&app, &mut model).await.unwrap();
    let config = model.selected_config.unwrap();
    assert_eq!(config.concurrency, 13);
    assert_eq!(config.rps, 4.5);
    assert_eq!(config.batch_size, 2);
    assert_eq!(config.validation, "length");
}

#[test]
fn compact_error_preserves_exit_and_hidden_fields_are_discoverable() {
    let (_dir, app, _) = setup();
    let mut model = Model::new();
    model.home = false;
    model.panel = 1;
    model.show_error(&Error::ThreadsBusy);
    let text = screen(&model, 40, 18);
    assert!(text.contains("Q quit"));
    assert!(text.contains("Esc jobs"));
    model.settings = Some(settings::Settings::new(&app, &model, 0).unwrap());
    model.error = None;
    model.message.clear();
    let text = screen(&model, 40, 18);
    assert!(text.contains("3/4 fields"));
    assert!(text.contains("Save API + Secrets"));
}

#[tokio::test]
async fn invalid_api_profile_is_repairable_and_validation_stays_in_its_group() {
    let (_dir, app, _) = setup();
    let mut model = Model::new();
    std::fs::write(app.paths.config_file(), "corrupt-synthetic-private").unwrap();
    run_repl_line(&app, &mut model, "settings-view")
        .await
        .unwrap();
    assert!(screen(&model, 80, 24).contains("Saved API profile is invalid"));
    assert!(!screen(&model, 80, 24).contains("corrupt-synthetic-private"));
    apply_key(&mut model, key(KeyCode::Tab));
    apply_paste(&mut model, "repaired-client");
    apply_key(&mut model, key(KeyCode::F(2)));
    apply_key(&mut model, key(KeyCode::Tab));
    apply_paste(&mut model, "replacement-synthetic-secret");
    run_repl_line(&app, &mut model, "save-settings")
        .await
        .unwrap();
    assert_eq!(app.api_config().unwrap().client_id, "repaired-client");
    apply_key(&mut model, key(KeyCode::F(2)));
    model.settings.as_mut().unwrap().threads = "0".into();
    let error = run_repl_line(&app, &mut model, "save-settings")
        .await
        .unwrap_err();
    model.show_error(&error);
    assert!(screen(&model, 40, 18).contains("Invalid threads"));
    apply_key(&mut model, key(KeyCode::F(2)));
    assert!(!screen(&model, 40, 18).contains("Invalid threads"));
    apply_key(
        &mut model,
        KeyEvent::new(KeyCode::F(2), KeyModifiers::SHIFT),
    );
    assert!(screen(&model, 40, 18).contains("Invalid threads"));
}

#[test]
fn help_accepts_the_primary_actions_still_displayed_in_its_footer() {
    for home in [true, false] {
        for (ch, command) in [
            ('q', "quit"),
            ('h', "home-view"),
            ('s', "settings-view"),
            ('n', "import-form"),
        ] {
            let mut model = Model::new();
            model.home = home;
            model.help = true;
            assert_eq!(
                apply_key(&mut model, key(KeyCode::Char(ch))).as_deref(),
                Some(command)
            );
            assert!(!model.help);
        }
    }
}

#[test]
fn focused_settings_field_keeps_its_label_visible() {
    let (_dir, app, _) = setup();
    let mut model = Model::new();
    model.settings = Some(settings::Settings::new(&app, &model, 0).unwrap());
    apply_key(&mut model, key(KeyCode::Tab));
    let mut terminal = ratatui::Terminal::new(ratatui::backend::TestBackend::new(40, 18)).unwrap();
    terminal.draw(|f| render(f, &model)).unwrap();
    let label = terminal
        .backend()
        .buffer()
        .content()
        .iter()
        .find(|c| c.symbol() == ">")
        .expect("focus label");
    assert_ne!(label.fg, label.bg, "focused field label must stay visible");
}

#[tokio::test]
async fn invalid_processing_defaults_are_identified_until_replaced() {
    let (_dir, app, _) = setup();
    std::fs::write(
        app.paths.root.join("processing.json"),
        "invalid-private-content",
    )
    .unwrap();
    let mut model = Model::new();
    run_repl_line(&app, &mut model, "default-threads-form")
        .await
        .unwrap();
    assert!(screen(&model, 80, 24).contains("Saved processing defaults are invalid"));
    assert!(!screen(&model, 80, 24).contains("invalid-private-content"));
    run_repl_line(&app, &mut model, "save-settings")
        .await
        .unwrap();
    assert_eq!(app.processing_defaults().unwrap().threads, 8);
    assert!(!screen(&model, 80, 24).contains("Saved processing defaults are invalid"));
}
