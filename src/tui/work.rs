use super::Model;
use crate::engine::{App, ChecksumReport, OutFormat};
use crate::error::Error;
use crate::jobcfg::JobConfig;

pub(super) enum Work {
    Import(std::path::PathBuf, Box<JobConfig>),
    Export {
        id: String,
        format: OutFormat,
        partial: bool,
    },
    Checksum(String),
    Prepare(String),
}

pub(super) struct WorkResult {
    pub selected: String,
    pub message: String,
    pub lines: Vec<String>,
    pub checksum: Option<ChecksumReport>,
    pub imported: bool,
}

impl Work {
    pub fn title(&self) -> &'static str {
        match self {
            Self::Import(..) => "Importing CSV",
            Self::Export { .. } => "Exporting file",
            Self::Checksum(_) => "Checking SHA-256",
            Self::Prepare(_) => "Preparing CSV",
        }
    }
    pub fn execute(self, app: &App) -> Result<WorkResult, Error> {
        match self {
            Self::Import(path, config) => {
                let id = app.create_job(&path, *config, None)?;
                Ok(WorkResult {
                    selected: id,
                    message: "Job created · R: run.".into(),
                    lines: vec![format!("Imported: {}", path.display())],
                    checksum: None,
                    imported: true,
                })
            }
            Self::Prepare(id) => {
                app.prepare_job(&id, None)?;
                Ok(WorkResult {
                    selected: id,
                    message: "CSV ready · R: run.".into(),
                    lines: vec![],
                    checksum: None,
                    imported: false,
                })
            }
            Self::Export {
                id,
                format,
                partial,
            } => {
                let report = app.merge(&id, format, partial, false)?;
                let path = app
                    .paths
                    .job_dir(&report.job_id)
                    .join("exports")
                    .join(format!(
                        "gen-{:04}/merged.{}",
                        report.generation, report.format
                    ));
                Ok(WorkResult {
                    selected: report.job_id,
                    message: if report.partial {
                        "Partial export · Events: file path.".into()
                    } else {
                        "Exported · Events: path and hash.".into()
                    },
                    lines: vec![
                        format!(
                            "Export: {} rows{}.",
                            report.rows,
                            if report.partial {
                                " (PARTIAL: includes UNKNOWN)"
                            } else {
                                ""
                            }
                        ),
                        format!("File: {}", path.display()),
                        format!("SHA-256: {}", report.sha256),
                    ],
                    checksum: None,
                    imported: false,
                })
            }
            Self::Checksum(id) => {
                let report = app.verify_checksum(&id)?;
                Ok(WorkResult {
                    selected: report.job_id.clone(),
                    message: if report.matches() {
                        "SHA-256 MATCH · files agree.".into()
                    } else {
                        "SHA-256 MISMATCH · N: new job.".into()
                    },
                    lines: vec![],
                    checksum: Some(report),
                    imported: false,
                })
            }
        }
    }
}

pub(super) fn accept(model: &mut Model, result: WorkResult) {
    if result.imported {
        model.import_draft = None;
    }
    model.message_job = Some(result.selected.clone());
    model.home = false;
    model.selected_job = Some(result.selected);
    model.message = result.message;
    model.error = None;
    for line in result.lines {
        model.push_event(line);
    }
    if let Some(report) = result.checksum {
        model.checksum = Some(report);
        model.panel = 4;
    } else {
        model.panel = 1;
    }
    model.scroll = 0;
    model.push_event(model.message.clone());
}

/// Exercises the same queued operations as the event loop, without requiring a terminal.
pub async fn perform_pending_work(app: &App, model: &mut Model) -> Result<(), Error> {
    if let Some(work) = model.pending_work.take() {
        let owned = app.clone();
        let result = tokio::task::spawn_blocking(move || work.execute(&owned))
            .await
            .map_err(|_| Error::Msg("Local operation interrupted.".into()))??;
        accept(model, result);
        super::refresh(app, model).await?;
    }
    Ok(())
}
