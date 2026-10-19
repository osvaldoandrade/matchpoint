//! Reproducible, network-free benchmark of the active matching pipeline.
use clap::Parser;
use pgm_data::{
    matching::{format::Bloom, Rule, RunControl, Service, Snapshot},
    secrets::MemorySecrets,
};
use std::{
    fs::{self, File},
    io::{BufRead, BufReader, BufWriter, Write},
    path::PathBuf,
    sync::Arc,
    time::Instant,
};
#[derive(Parser)]
struct Args {
    #[arg(long, default_value_t = 1_000_000)]
    rows: u64,
    #[arg(long)]
    output: Option<PathBuf>,
    #[arg(long, default_value_t=1, value_parser=clap::value_parser!(u16).range(1..=256))]
    threads: u16,
}
#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args = Args::parse();
    if args.rows == 0 {
        return Err("rows must be positive".into());
    }
    let root = args.output.unwrap_or_else(|| {
        std::env::temp_dir().join(format!("pgm-match-bench-{}", uuid::Uuid::new_v4()))
    });
    fs::create_dir_all(&root)?;
    let rule = Rule::default();
    let started = Instant::now();
    let input = root.join("synthetic.csv");
    let mut file = BufWriter::new(File::create(&input)?);
    pgm_data::error::restrict_file(&input)?;
    writeln!(file, "identifier")?;
    let mut bloom = Bloom::new(args.rows.div_ceil(10), &rule)?;
    for i in 0..args.rows {
        let key = format!("{i:011}");
        writeln!(file, "{key}")?;
        if i % 10 == 0 {
            bloom.insert(&key)?;
        }
    }
    file.flush()?;
    drop(file);
    let dat = root.join("synthetic.dat");
    bloom.write(&dat)?;
    drop(bloom);
    let snapshot = Snapshot {
        bloom: Arc::new(Bloom::read(&dat, 512 * 1024 * 1024)?),
    };
    let service = Service::open(root.join("state"), Arc::new(MemorySecrets::new()))?;
    let prepare = Instant::now();
    let job = service.create_job(&input, rule)?;
    let prepare_ms = prepare.elapsed().as_millis();
    let run = Instant::now();
    let mut control = RunControl::default();
    control.threads = Some(args.threads);
    let job = service
        .run_job(&job.id, Some(snapshot.clone()), control)
        .await?;
    let seconds = run.elapsed().as_secs_f64();
    let mut rows = 0;
    let mut false_negatives = 0;
    for line in BufReader::new(File::open(service.result_path(&job.id)?)?).lines() {
        let line = line?;
        let expected = rows % 10 == 0;
        if expected && !line.ends_with(": true") {
            false_negatives += 1;
        }
        rows += 1;
    }
    assert_eq!(rows, args.rows);
    assert_eq!(false_negatives, 0);
    assert!(service.verify(&job.id)?);
    println!(
        "{}",
        serde_json::to_string_pretty(
            &serde_json::json!({"rows":rows,"threads":args.threads,"prepare_ms":prepare_ms,"run_seconds":seconds,"rows_per_second":rows as f64/seconds,"total_seconds":started.elapsed().as_secs_f64(),"filter_bytes":snapshot.bloom.info().bytes,"dat_bytes":fs::metadata(dat)?.len(),"true":job.positives,"false":job.negatives,"false_negatives":false_negatives,"false_positives":job.positives-args.rows.div_ceil(10),"checksum_match":true,"http_requests":0,"artifacts":root})
        )?
    );
    Ok(())
}
