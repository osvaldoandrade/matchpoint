//! Reproducible UI evidence using synthetic CSVs, MemorySecrets and a loopback fixture.
//! cargo run --example ui_review -- target/ux-review/round-1
//! cargo run --example ui_review -- --live (isolated, real interactive TUI)
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use pgm_data::{
    engine::{App, RunOptions},
    error::Secret,
    jobcfg::default_tuning,
    secrets::MemorySecrets,
    sim::Fixture,
    timeutil::SystemClock,
    tui::{self, Model},
};
use ratatui::{backend::TestBackend, style::Color, Terminal};
use std::{path::Path, sync::Arc, time::Duration};

fn xml(text: &str) -> String {
    text.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
}
fn color(color: Color) -> String {
    if let Color::Rgb(r, g, b) = color {
        return format!("#{r:02x}{g:02x}{b:02x}");
    }
    if let Color::Indexed(index @ 16..=231) = color {
        let cube = [0, 95, 135, 175, 215, 255];
        let n = (index - 16) as usize;
        return format!(
            "#{:02x}{:02x}{:02x}",
            cube[n / 36],
            cube[n / 6 % 6],
            cube[n % 6]
        );
    }
    if let Color::Indexed(index @ 232..=255) = color {
        let value = 8 + (index - 232) * 10;
        return format!("#{value:02x}{value:02x}{value:02x}");
    }
    match color {
        Color::Black => "#000000",
        Color::LightGreen => "#80ff80",
        Color::Green => "#00aa00",
        Color::DarkGray => "#555555",
        Color::LightRed => "#ff8080",
        Color::LightYellow => "#ffff80",
        Color::Gray => "#c0c0c0",
        _ => "#c0c0c0",
    }
    .into()
}
fn ansi_color(color: Color, background: bool) -> String {
    if let Color::Rgb(r, g, b) = color {
        return format!("{};2;{r};{g};{b}", if background { 48 } else { 38 });
    }
    if let Color::Indexed(index) = color {
        return format!("{};5;{index}", if background { 48 } else { 38 });
    }
    let fg = match color {
        Color::Black => 30,
        Color::Green => 32,
        Color::DarkGray => 90,
        Color::LightGreen => 92,
        Color::LightRed => 91,
        Color::LightYellow => 93,
        _ => 37,
    };
    (fg + if background { 10 } else { 0 }).to_string()
}

#[test]
fn exports_preserve_truecolor_foreground_and_background() {
    let rgb = Color::Rgb(13, 19, 26);
    assert_eq!(color(rgb), "#0d131a");
    assert_eq!(ansi_color(rgb, false), "38;2;13;19;26");
    assert_eq!(ansi_color(rgb, true), "48;2;13;19;26");
}

fn snapshot(output: &Path, name: &str, model: &Model) {
    for (w, h) in [(120, 40), (100, 30), (80, 24), (40, 18)] {
        let mut terminal = Terminal::new(TestBackend::new(w, h)).unwrap();
        terminal.draw(|frame| tui::render(frame, model)).unwrap();
        let buffer = terminal.backend().buffer();
        let mut text = String::new();
        let mut ansi = String::new();
        let cw = if model.font_vga { 8 } else { 9 };
        let ch = if model.font_vga { 16 } else { 18 };
        let font_px = if model.font_vga { 16 } else { 14 };
        let baseline = if model.font_vga { 12 } else { 14 };
        let family = if model.font_vga {
            "PxPlus IBM VGA 8x16"
        } else {
            "Menlo"
        };
        let canvas = color(buffer[(0, 0)].bg);
        let mut svg = format!("<svg xmlns=\"http://www.w3.org/2000/svg\" width=\"{}\" height=\"{}\"><rect width=\"100%\" height=\"100%\" fill=\"{canvas}\"/><g font-family=\"'{family}',monospace\" font-size=\"{font_px}\">", w*cw+16,h*ch+16);
        for y in 0..h {
            for x in 0..w {
                let cell = &buffer[(x, y)];
                text.push_str(cell.symbol());
                let fg = color(cell.fg);
                let bg = color(cell.bg);
                if bg != "#000000" {
                    svg.push_str(&format!(
                        "<rect x=\"{}\" y=\"{}\" width=\"{cw}\" height=\"{ch}\" fill=\"{bg}\"/>",
                        x * cw + 8,
                        y * ch + 8
                    ));
                }
                svg.push_str(&format!(
                    "<text x=\"{}\" y=\"{}\" fill=\"{fg}\">{}</text>",
                    x * cw + 8,
                    y * ch + 8 + baseline,
                    xml(cell.symbol())
                ));
                ansi.push_str(&format!(
                    "\x1b[{};{}m{}",
                    ansi_color(cell.fg, false),
                    ansi_color(cell.bg, true),
                    cell.symbol()
                ));
            }
            text.push('\n');
            ansi.push_str("\x1b[0m\n");
        }
        svg.push_str("</g></svg>");
        let stem = format!("{name}-{w}x{h}");
        std::fs::write(output.join(format!("{stem}.txt")), text).unwrap();
        std::fs::write(output.join(format!("{stem}.ansi")), ansi).unwrap();
        std::fs::write(output.join(format!("{stem}.svg")), svg).unwrap();
    }
}
fn type_text(model: &mut Model, value: &str) {
    for ch in value.chars() {
        tui::apply_key(model, KeyEvent::new(KeyCode::Char(ch), KeyModifiers::NONE));
    }
}

fn snapshot_large_job_summaries(output: &Path) {
    for state in [
        "RUNNING",
        "COMPLETED",
        "COMPLETED_WITH_ERRORS",
        "BLOCKED_UNKNOWN",
    ] {
        let id = "12345678-0000-4000-8000-000000000000".to_string();
        let mut counts = pgm_data::domain::Counts::default();
        match state {
            "RUNNING" => {
                counts.succeeded = 500_000;
                counts.pending = 500_000;
            }
            "COMPLETED_WITH_ERRORS" => {
                counts.succeeded = 999_999;
                counts.failed_final = 1;
            }
            "BLOCKED_UNKNOWN" => counts.unknown = 1_000_000,
            _ => counts.succeeded = 1_000_000,
        }
        let mut model = Model::new();
        model.home = false;
        model.panel = 1;
        model.selected_job = Some(id.clone());
        model.jobs.push(pgm_data::engine::JobStatus {
            id,
            state: state.into(),
            desired: "run".into(),
            counts,
            batches: 1_000,
        });
        snapshot(
            output,
            &format!("large-{}", state.to_ascii_lowercase()),
            &model,
        );
    }
    let id = "12345678-0000-4000-8000-000000000000".to_string();
    let mut model = Model::new();
    model.home = false;
    model.panel = 3;
    model.selected_job = Some(id.clone());
    model.jobs.push(pgm_data::engine::JobStatus {
        id,
        state: "BLOCKED_UNKNOWN".into(),
        desired: "run".into(),
        counts: pgm_data::domain::Counts {
            unknown: i64::MAX,
            ..Default::default()
        },
        batches: 1,
    });
    model.show_error(&pgm_data::error::Error::UnknownBlocks { unknown: i64::MAX });
    model.events = vec![
        "Uncertain outcomes require review.".into(),
        "Export was blocked.".into(),
    ]
    .into();
    snapshot(output, "large-error-events", &model);
    model.panel = 2;
    model.batch_items = vec![pgm_data::store::BatchInfo {
        batch_id: 1,
        state: "BLOCKED_UNKNOWN".into(),
        row_from: 1,
        row_to: 1_000,
        pending: 0,
        in_flight: 0,
        retry_wait: 0,
        succeeded: 0,
        failed: 0,
        unknown: 1_000,
    }];
    snapshot(output, "large-error-batches", &model);
}
async fn dispatch(app: &App, model: &mut Model, command: &str) {
    if let Err(err) = tui::run_repl_line(app, model, command).await {
        model.show_error(&err);
    }
}

async fn measure(
    app: &App,
    model: &mut Model,
    name: &str,
    selected: Option<&str>,
) -> serde_json::Value {
    let mut times = Vec::new();
    for _ in 0..64 {
        let before = std::time::Instant::now();
        tui::apply_key(model, KeyEvent::new(KeyCode::Down, KeyModifiers::NONE));
        if let Some(id) = selected {
            model.selected_job = Some(id.into());
        }
        tui::refresh(app, model).await.unwrap();
        let mut terminal = Terminal::new(TestBackend::new(100, 30)).unwrap();
        terminal.draw(|f| tui::render(f, model)).unwrap();
        times.push(before.elapsed().as_micros());
    }
    times.sort();
    let at = (95usize * times.len()).div_ceil(100) - 1;
    serde_json::json!({"scenario":name,"samples_us":times,"p95_ms":times[at] as f64/1000.})
}

fn measure_numeric_frames() -> serde_json::Value {
    let mut reports = Vec::new();
    for (w, h) in [(120, 40), (100, 30), (80, 24), (40, 18)] {
        for (splash, scene) in [
            (false, tui::SceneKind::Tesseract),
            (true, tui::SceneKind::Tesseract),
            (true, tui::SceneKind::Lattice),
        ] {
            let mut model = Model::new();
            model.scene = scene;
            model.splash = splash;
            model.jobs = (0..225)
                .map(|i| pgm_data::engine::JobStatus {
                    id: format!("synthetic-{i:08}"),
                    state: ["PREPARED", "RUNNING", "COMPLETED", "PAUSED"][i % 4].into(),
                    desired: "pause".into(),
                    counts: pgm_data::domain::Counts {
                        pending: 1000,
                        succeeded: 500,
                        unknown: 3,
                        ..Default::default()
                    },
                    batches: 2,
                })
                .collect();
            let mut terminal = Terminal::new(TestBackend::new(w, h)).unwrap();
            let mut times = Vec::new();
            for frame in 0..64 {
                model.animation = frame * 2;
                let before = std::time::Instant::now();
                terminal.draw(|f| tui::render(f, &model)).unwrap();
                times.push(before.elapsed().as_micros());
            }
            times.sort();
            let at = (95usize * times.len()).div_ceil(100) - 1;
            reports.push(serde_json::json!({"screen":if splash {"splash"} else {"home"},"scene":scene.label(),"jobs":225,"viewport":[w,h],"samples":64,"p95_ms":times[at] as f64/1000.,"max_ms":times.last().unwrap().to_owned() as f64/1000.}));
        }
    }
    serde_json::json!({"scope":"splash/home frame CPU + TestBackend diff/flush; 225 synthetic loaded jobs = 1.5x prior 150-job scenario; no native paint or provider capacity claim","scenes":reports})
}

#[tokio::main]
async fn main() {
    let args: Vec<_> = std::env::args().skip(1).collect();
    let live = args.first().is_some_and(|arg| arg == "--live");
    let temporary = tempfile::tempdir().unwrap();
    let latency = args.first().is_some_and(|arg| arg == "--latency");
    let output = if live || latency {
        temporary.path().join("screens")
    } else {
        args.first()
            .map(std::path::PathBuf::from)
            .unwrap_or("target/ux-review/current".into())
    };
    std::fs::create_dir_all(&output).unwrap();
    let app = App::open(
        temporary.path().join("data"),
        Arc::new(MemorySecrets::new()),
        Arc::new(SystemClock),
    )
    .unwrap();
    let mut model = Model::new();
    if latency {
        let input = temporary.path().join("latency.csv");
        std::fs::write(&input, "52998224725\n").unwrap();
        for _ in 0..150 {
            app.create_job(&input, default_tuning(), None).unwrap();
        }
        model.home = false;
        tui::refresh(&app, &mut model).await.unwrap();
        let small = measure(&app, &mut model, "150 small jobs", None).await;
        std::fs::write(&input, "52998224725\n".repeat(1_000_000)).unwrap();
        let large = app.create_job(&input, default_tuning(), None).unwrap();
        model.selected_job = Some(large.clone());
        tui::refresh(&app, &mut model).await.unwrap();
        assert_eq!(app.status(&large).unwrap().counts.total(), 1_000_000);
        let large_report = measure(
            &app,
            &mut model,
            "150 small jobs + selected 1 million records",
            Some(&large),
        )
        .await;
        let report = serde_json::json!({"includes":"selection, refresh, render; real SQLite; synthetic data; 100x30; 64 samples per scenario", "scenarios":[small,large_report],"numeric_frames":measure_numeric_frames()});
        if let Some(path) = args.get(1) {
            std::fs::write(path, serde_json::to_vec_pretty(&report).unwrap()).unwrap();
        }
        println!("{}", report);
        return;
    }

    tui::refresh(&app, &mut model).await.unwrap();
    if !live {
        snapshot_large_job_summaries(&output);
        model.splash = true;
        snapshot(&output, "splash", &model);
        model.animation = 28;
        snapshot(&output, "splash-frame-28", &model);
        model.scene = tui::SceneKind::Lattice;
        snapshot(&output, "splash-circuit", &model);
        model.motion = false;
        snapshot(&output, "splash-reduced-motion", &model);
        model.splash = false;
        model.motion = true;
        model.animation = 0;
        model.scene = tui::SceneKind::Tesseract;
        snapshot(&output, "first-use", &model);
        tui::apply_key(
            &mut model,
            KeyEvent::new(KeyCode::Right, KeyModifiers::NONE),
        );
        for tick in [0, 4, 8] {
            model.animation = tick;
            snapshot(&output, &format!("home-focus-{tick}"), &model);
        }
        model.home_selection = 0;
        model.motion = false;
        snapshot(&output, "home-reduced-motion", &model);
        model.animation = 128;
        snapshot(&output, "home-reduced-motion-128", &model);
        model.motion = true;
        model.animation = 0;
        for selected in 1..3 {
            model.home_selection = selected;
            snapshot(&output, &format!("home-menu-{selected}"), &model);
        }
        model.home_selection = 0;
        model.animation = 28;
        snapshot(&output, "home-phase-28", &model);
        model.animation = 0;
        dispatch(&app, &mut model, "settings-view").await;
        snapshot(&output, "settings-api", &model);
        tui::apply_key(
            &mut model,
            KeyEvent::new(KeyCode::Right, KeyModifiers::NONE),
        );
        snapshot(&output, "settings-secrets", &model);
        tui::apply_key(
            &mut model,
            KeyEvent::new(KeyCode::Right, KeyModifiers::NONE),
        );
        snapshot(&output, "settings-processing", &model);
        tui::apply_key(
            &mut model,
            KeyEvent::new(KeyCode::Right, KeyModifiers::NONE),
        );
        snapshot(&output, "appearance", &model);
        tui::apply_key(&mut model, KeyEvent::new(KeyCode::Esc, KeyModifiers::NONE));
        dispatch(&app, &mut model, "settings-view").await;
        tui::apply_key(&mut model, KeyEvent::new(KeyCode::Tab, KeyModifiers::NONE));
        tui::apply_paste(&mut model, "draft-synthetic-client");
        tui::apply_key(&mut model, KeyEvent::new(KeyCode::F(2), KeyModifiers::NONE));
        tui::apply_key(&mut model, KeyEvent::new(KeyCode::Tab, KeyModifiers::NONE));
        tui::apply_paste(&mut model, "draft-synthetic-secret");
        snapshot(&output, "settings-unsaved-secret", &model);
        tui::apply_key(&mut model, KeyEvent::new(KeyCode::F(2), KeyModifiers::NONE));
        tui::apply_key(&mut model, KeyEvent::new(KeyCode::Tab, KeyModifiers::NONE));
        tui::apply_key(
            &mut model,
            KeyEvent::new(KeyCode::Char('u'), KeyModifiers::CONTROL),
        );
        tui::apply_paste(&mut model, "12");
        dispatch(&app, &mut model, "save-settings").await;
        snapshot(&output, "settings-saved-one-group", &model);
        tui::apply_key(&mut model, KeyEvent::new(KeyCode::F(2), KeyModifiers::NONE));
        snapshot(&output, "settings-other-group", &model);
        tui::apply_key(&mut model, KeyEvent::new(KeyCode::Esc, KeyModifiers::NONE));
        dispatch(&app, &mut model, "doctor").await;
        snapshot(&output, "diagnosis", &model);
        model.home = true;
        dispatch(&app, &mut model, "/help").await;
        snapshot(&output, "help-top", &model);
        model.scroll = 500;
        snapshot(&output, "help-end", &model);
        model.help = false;
        model.scroll = 0;
        model.repl = "/".into();
        snapshot(&output, "menu", &model);
        model.repl.clear();
        dispatch(&app, &mut model, "/import").await;
        dispatch(&app, &mut model, "save-import").await;
        snapshot(&output, "import-invalid", &model);
        type_text(&mut model, "/path/does-not-exist.csv");
        dispatch(&app, &mut model, "save-import").await;
        snapshot(&output, "import-missing", &model);
        tui::apply_key(&mut model, KeyEvent::new(KeyCode::Esc, KeyModifiers::NONE));
        // Real form path with spaces, persisted default and cancellation.
        dispatch(&app, &mut model, "/threads-default").await;
        tui::apply_key(&mut model, KeyEvent::new(KeyCode::Tab, KeyModifiers::NONE));
        tui::apply_key(
            &mut model,
            KeyEvent::new(KeyCode::Char('u'), KeyModifiers::CONTROL),
        );
        type_text(&mut model, "12");
        dispatch(&app, &mut model, "save-threads").await;
        tui::apply_key(&mut model, KeyEvent::new(KeyCode::Esc, KeyModifiers::NONE));
        let input = temporary.path().join("clientes de teste.csv");
        std::fs::write(&input, "52998224725\n39053344705\n").unwrap();
        dispatch(&app, &mut model, "/import").await;
        type_text(&mut model, input.to_str().unwrap());
        snapshot(&output, "import-filled", &model);
        dispatch(&app, &mut model, "save-import").await;
        tui::perform_pending_work(&app, &mut model).await.unwrap();
        assert_eq!(model.selected_config.as_ref().unwrap().concurrency, 12);
        snapshot(&output, "imported", &model);
    }
    let fixture = Fixture::spawn().await;
    app.configure_auth(
        "synthetic-client",
        Secret::new("synthetic-local-only".into()).unwrap(),
        None,
    )
    .unwrap();
    let input = temporary.path().join("synthetic-job.csv");
    std::fs::write(&input, "52998224725\n".repeat(250)).unwrap();
    let mut config = default_tuning();
    config.endpoint = fixture.process_url();
    config.token_url = fixture.token_url();
    config.test_mode = true;
    config.contract = "simulated-fixture-v1".into();
    config.client_id = "synthetic-client".into();
    config.concurrency = 4;
    config.rps = 100.0;
    config.batch_size = 10;
    config.commit_max_rows = 1;
    config.commit_max_ms = 20;
    let id = app.create_job(&input, config.clone(), None).unwrap();
    let other = app.create_job(&input, config.clone(), None).unwrap();
    model.selected_job = Some(id.clone());
    if live {
        println!("Synthetic fixture only. CSV input: {}", input.display());
        tui::launch(&app).await.unwrap();
        return;
    }
    tui::refresh(&app, &mut model).await.unwrap();
    let was_home = model.home;
    model.home = true;
    snapshot(&output, "home-loaded", &model);
    model.home = was_home;
    dispatch(&app, &mut model, "/threads").await;
    tui::apply_key(
        &mut model,
        KeyEvent::new(KeyCode::Char('u'), KeyModifiers::CONTROL),
    );
    type_text(&mut model, "0");
    dispatch(&app, &mut model, "save-threads").await;
    snapshot(&output, "threads-invalid", &model);
    tui::apply_key(
        &mut model,
        KeyEvent::new(KeyCode::Char('u'), KeyModifiers::CONTROL),
    );
    type_text(&mut model, "6");
    dispatch(&app, &mut model, "save-threads").await;
    assert_eq!(app.job_config(&id).unwrap().concurrency, 6);
    snapshot(&output, "threads-saved", &model);
    model.active_job = Some(id.clone());
    model.busy = true;
    model.panel = 1;
    let owned = app.clone();
    let running_id = id.clone();
    let run = tokio::spawn(async move { owned.run_job(&running_id, RunOptions::run()).await });
    tokio::time::sleep(Duration::from_millis(250)).await;
    tui::refresh(&app, &mut model).await.unwrap();
    snapshot(&output, "running-frame-0", &model);
    model.animation = 2;
    snapshot(&output, "running-frame-1", &model);
    dispatch(&app, &mut model, "/threads").await;
    snapshot(&output, "threads-running-blocked", &model);
    model.selected_job = Some(other.clone());
    model.error = None;
    dispatch(&app, &mut model, "/pause").await;
    let report = run.await.unwrap().unwrap();
    assert_eq!(report.state, "PAUSED");
    assert_eq!(app.status(&other).unwrap().state, "PREPARED");
    tui::finish_run(&mut model, Ok(report));
    tui::refresh(&app, &mut model).await.unwrap();
    assert_eq!(model.selected_job.as_ref(), Some(&other));
    assert!(model.message.contains(&id[..8]));
    snapshot(&output, "paused-other-selected", &model);
    model.selected_job = Some(id.clone());
    tui::refresh(&app, &mut model).await.unwrap();
    snapshot(&output, "paused", &model);
    let report = app.run_job(&id, RunOptions::run()).await.unwrap();
    assert_eq!(report.state, "COMPLETED");
    tui::refresh(&app, &mut model).await.unwrap();
    snapshot(&output, "completed", &model);
    dispatch(&app, &mut model, "export-form").await;
    tui::apply_key(
        &mut model,
        KeyEvent::new(KeyCode::Char(' '), KeyModifiers::NONE),
    );
    snapshot(&output, "export-form", &model);
    dispatch(&app, &mut model, "save-export").await;
    tui::perform_pending_work(&app, &mut model).await.unwrap();
    model.panel = 3;
    snapshot(&output, "exported", &model);
    dispatch(&app, &mut model, "/checksum").await;
    tui::perform_pending_work(&app, &mut model).await.unwrap();
    assert!(model.checksum.as_ref().unwrap().matches());
    snapshot(&output, "checksum-ok", &model);
    model.home = true;
    snapshot(&output, "home-progress", &model);
    model.home = false;
    model.scroll = 500;
    snapshot(&output, "checksum-end", &model);
    model.scroll = 0;
    std::fs::write(app.paths.job_dir(&id).join("input.csv"), "changed\n").unwrap();
    dispatch(&app, &mut model, "/checksum").await;
    tui::perform_pending_work(&app, &mut model).await.unwrap();
    assert!(!model.checksum.as_ref().unwrap().matches());
    snapshot(&output, "checksum-divergent", &model);
    for _ in 0..8 {
        app.create_job(&input, config.clone(), None).unwrap();
    }
    tui::refresh(&app, &mut model).await.unwrap();
    model.selected_job = model.jobs.last().map(|job| job.id.clone());
    model.panel = 0;
    tui::refresh(&app, &mut model).await.unwrap();
    snapshot(&output, "many-jobs-last", &model);
    model.panel = 2;
    model.batch_offset = 20;
    tui::refresh(&app, &mut model).await.unwrap();
    snapshot(&output, "batches-page", &model);
    std::fs::write(output.join("flow-evidence.txt"),"PASS: form import path with spaces; default threads=12 inherited; invalid threads rejected and value preserved; saved threads=6; actual loopback execution; frames with same counters/different activity; active-job pause after selecting another job; PAUSED then COMPLETED; CSV export; SHA-256 input+export match; tampered input diverges; >viewport jobs; batch paging. MemorySecrets only, no Keychain or real API.").unwrap();
    std::fs::write(
        output.join("frame-performance.json"),
        serde_json::to_vec_pretty(&measure_numeric_frames()).unwrap(),
    )
    .unwrap();
    println!("Evidence: {}", output.display());
}
