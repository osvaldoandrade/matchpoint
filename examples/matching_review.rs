//! Synthetic render evidence for the active workflow; no user data or credentials.
use pgm_data::{
    matching::{
        format::Bloom,
        ui::{self, Page, Ui},
        Phase, Progress, Rule, RunControl, Service, Settings, Snapshot,
    },
    secrets::MemorySecrets,
};
use ratatui::{backend::TestBackend, style::Color, Terminal};
use std::{path::Path, sync::Arc};
fn color(color: Color) -> String {
    if let Color::Rgb(r, g, b) = color {
        format!("#{r:02x}{g:02x}{b:02x}")
    } else {
        "#cfdde3".into()
    }
}
fn xml(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
}
fn snapshot(root: &Path, name: &str, ui: &Ui) {
    for (w, h) in [(40, 18), (60, 22), (80, 24), (120, 36)] {
        let mut terminal = Terminal::new(TestBackend::new(w, h)).unwrap();
        terminal.draw(|f| ui::render(f, ui)).unwrap();
        let buffer = terminal.backend().buffer();
        let mut text = String::new();
        let mut svg=format!("<svg xmlns=\"http://www.w3.org/2000/svg\" width=\"{}\" height=\"{}\"><rect width=\"100%\" height=\"100%\" fill=\"#0d131a\"/><g font-family=\"'PxPlus IBM VGA 8x16',monospace\" font-size=\"16\">",w*8+16,h*16+16);
        for y in 0..h {
            for x in 0..w {
                let cell = &buffer[(x, y)];
                text.push_str(cell.symbol());
                svg.push_str(&format!("<rect x=\"{}\" y=\"{}\" width=\"8\" height=\"16\" fill=\"{}\"/><text x=\"{}\" y=\"{}\" fill=\"{}\">{}</text>",x*8+8,y*16+8,color(cell.bg),x*8+8,y*16+20,color(cell.fg),xml(cell.symbol())));
            }
            text.push('\n');
        }
        svg.push_str("</g></svg>");
        std::fs::write(root.join(format!("{name}-{w}x{h}.txt")), text).unwrap();
        std::fs::write(root.join(format!("{name}-{w}x{h}.svg")), svg).unwrap();
    }
}
#[tokio::main]
async fn main() {
    let output = std::env::args()
        .nth(1)
        .unwrap_or_else(|| "target/bloom-review".into());
    let output = Path::new(&output);
    std::fs::create_dir_all(output).unwrap();
    let data = output.join(uuid::Uuid::new_v4().to_string());
    std::fs::create_dir(&data).unwrap();
    let service = Service::open(data.clone(), Arc::new(MemorySecrets::new())).unwrap();
    let mut ui = Ui::new(Settings::default());
    ui.data_root = Some(std::path::absolute(&service.root).unwrap());
    snapshot(output, "login", &ui);
    ui.values[1] = "synthetic-hidden-secret".into();
    ui.focus = Some(1);
    snapshot(output, "login-secret", &ui);
    ui.values[1].clear();
    ui.values[3] = "synthetic-aes-key-hidden-in-render".into();
    ui.focus = Some(3);
    snapshot(output, "login-aes", &ui);
    ui.values[3].clear();
    ui.failed = true;
    ui.message = "Sign in failed · HTTP 401".into();
    snapshot(output, "login-error", &ui);
    ui.failed = false;
    ui.busy = true;
    ui.message = "Signing in…".into();
    snapshot(output, "login-busy", &ui);
    ui.busy = false;
    // Dashboard previews use synthetic state; this does not authenticate a live session.
    ui.signed_in = true;
    ui.open(Page::Home);
    snapshot(output, "home-empty", &ui);
    let input = data.join("partners.csv");
    std::fs::write(
        &input,
        "identifier\ncustomer-000042\nproduct-0042\ncustomer-000042\nabsent\n",
    )
    .unwrap();
    let mut bloom = Bloom::new(1, &Rule::default()).unwrap();
    bloom.insert("customer-000042").unwrap();
    let dat = data.join("filter.dat");
    bloom.write(&dat).unwrap();
    let filter = Snapshot {
        bloom: Arc::new(Bloom::read(&dat, 1024).unwrap()),
    };
    let job = service
        .create_job_cancellable_to(&input, Rule::default(), &RunControl::default().stop, &data)
        .unwrap();
    service
        .run_job(&job.id, Some(filter.clone()), RunControl::default())
        .await
        .unwrap();
    ui.jobs = service.jobs().unwrap();
    drop(filter);
    snapshot(output, "home", &ui);
    for (page, name) in [
        (Page::NewJob, "new-job"),
        (Page::Jobs, "jobs"),
        (Page::Export, "export"),
        (Page::Threads, "threads"),
        (Page::Delete, "delete-job"),
        (Page::Settings, "settings"),
        (Page::Help, "help"),
    ] {
        ui.open(page);
        if page == Page::Help {
            ui.manual = Default::default();
        }
        snapshot(output, name, &ui);
    }
    ui.manual
        .key(crossterm::event::KeyCode::Left.into(), (80, 24));
    snapshot(output, "help-datasets", &ui);
    ui.open(Page::Jobs);
    ui.busy = true;
    ui.jobs[0].state = "PREPARED".into();
    ui.jobs[0].total = 20_000_000;
    ui.jobs[0].processed = 0;
    ui.jobs[0].positives = 0;
    ui.jobs[0].negatives = 0;
    for (phase, name) in [
        (Phase::WaitingForDat, "dat-wait"),
        (Phase::RetryWait, "dat-retry"),
        (Phase::DownloadingDat, "dat-transfer"),
        (Phase::VerifyingBloom, "dat-verify"),
        (Phase::Matching, "matching-progress"),
    ] {
        let mut p = Progress::new(phase);
        p.started -= std::time::Duration::from_secs(120);
        p.phase_started -= std::time::Duration::from_secs(60);
        p.retry = 1;
        p.retry_limit = 2;
        p.last_http = Some(504);
        p.retry_http = Some(504);
        p.completed = 10_000_000;
        p.total = Some(20_000_000);
        p.job = Some(uuid::Uuid::parse_str(&ui.jobs[0].id).unwrap());
        if phase == Phase::Matching {
            ui.jobs[0].state = "RUNNING".into();
            ui.jobs[0].processed = 9_900_000;
            ui.jobs[0].positives = 10_000;
            ui.jobs[0].negatives = 9_890_000;
        }
        ui.progress = Some(p);
        snapshot(output, name, &ui);
    }
    ui.busy = false;
    ui.progress = None;
    ui.open(Page::Settings);
    ui.config.download_mode = pgm_data::matching::DownloadMode::Parallel;
    ui.focus = Some(1);
    snapshot(output, "download-parallel", &ui);
    ui.config.download_mode = pgm_data::matching::DownloadMode::Multipart;
    ui.values[9] = r#"{"format":"dat"}"#.into();
    ui.focus = Some(8);
    snapshot(output, "download-form", &ui);
    ui.open(Page::Help);
    use crossterm::event::KeyCode;
    for key in std::iter::once(KeyCode::Char('/'))
        .chain("sha256".chars().map(KeyCode::Char))
        .chain(std::iter::once(KeyCode::Enter))
    {
        ui.manual.key(key.into(), (80, 24));
    }
    snapshot(output, "help-search", &ui);
    ui.manual
        .key(crossterm::event::KeyCode::End.into(), (80, 24));
    snapshot(output, "help-end", &ui);
    println!("Render artifacts: {}", output.display());
}
