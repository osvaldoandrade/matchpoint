use super::{App, MergeReport};
use crate::error::Error;
use crate::prepare::sha256_file;

#[derive(Clone, Debug)]
pub struct ChecksumArtifact {
    pub name: String,
    pub expected: String,
    pub actual: String,
    pub matches: bool,
}

#[derive(Clone, Debug)]
pub struct ChecksumReport {
    pub job_id: String,
    pub checked_at: String,
    pub artifacts: Vec<ChecksumArtifact>,
}

impl ChecksumReport {
    pub fn matches(&self) -> bool {
        self.artifacts.iter().all(|artifact| artifact.matches)
    }
}

impl App {
    /// Explicit verification of the stored input and each published merged export.
    /// Nothing is re-sent or rewritten by this operation.
    pub fn verify_checksum(&self, id: &str) -> Result<ChecksumReport, Error> {
        let config = self.job_config(id)?;
        let dir = self.paths.job_dir(&config.job_id);
        let mut artifacts = vec![check(
            &dir.join("input.csv"),
            "input.csv".into(),
            config.input_sha256,
        )?];
        let exports = dir.join("exports");
        if exports.exists() {
            let mut generations: Vec<_> = std::fs::read_dir(exports)?.collect::<Result<_, _>>()?;
            generations.sort_by_key(|entry| entry.file_name());
            for generation in generations {
                let path = generation.path();
                let manifest = path.join("manifest.json");
                if !manifest.is_file() {
                    continue;
                }
                let report: MergeReport = serde_json::from_slice(&std::fs::read(manifest)?)
                    .map_err(|_| {
                        Error::Msg("Invalid manifest; run /doctor and inspect the export.".into())
                    })?;
                if report.job_id != config.job_id
                    || !matches!(report.format.as_str(), "csv" | "parquet")
                {
                    return Err(Error::Msg(
                        "Manifest belongs to another job or has an invalid format.".into(),
                    ));
                }
                let name = format!(
                    "{}/merged.{}",
                    generation.file_name().to_string_lossy(),
                    report.format
                );
                artifacts.push(check(
                    &path.join(format!("merged.{}", report.format)),
                    name,
                    report.sha256,
                )?);
            }
        }
        Ok(ChecksumReport {
            job_id: config.job_id,
            checked_at: self.now(),
            artifacts,
        })
    }
}

fn check(
    path: &std::path::Path,
    name: String,
    expected: String,
) -> Result<ChecksumArtifact, Error> {
    let actual = match sha256_file(path) {
        Ok(hash) => hash,
        Err(Error::Io(err)) if err.kind() == std::io::ErrorKind::NotFound => "FILE MISSING".into(),
        Err(err) => return Err(err),
    };
    let matches = expected == actual;
    Ok(ChecksumArtifact {
        name,
        expected,
        actual,
        matches,
    })
}
