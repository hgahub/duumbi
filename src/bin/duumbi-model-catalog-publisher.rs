//! Generates deterministic DUUMBI v1 model-catalog artifacts.

use std::fs;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result, bail};
use duumbi::agents::model_catalog_publisher::{CatalogPublisherInput, publish_model_catalog_v1};

const CATALOG_FILE: &str = "model-catalog.v1.json";
const SHA256_FILE: &str = "model-catalog.v1.sha256";

fn main() -> Result<()> {
    let args = PublisherArgs::parse(std::env::args().skip(1))?;
    let body = fs::read_to_string(&args.input)
        .with_context(|| format!("failed to read input {}", args.input.display()))?;
    let input: CatalogPublisherInput = serde_json::from_str(&body)
        .with_context(|| format!("failed to parse input {}", args.input.display()))?;
    let published = publish_model_catalog_v1(&input).context("failed to publish catalog")?;

    fs::create_dir_all(&args.out_dir).with_context(|| {
        format!(
            "failed to create output directory {}",
            args.out_dir.display()
        )
    })?;
    let catalog_path = args.out_dir.join(CATALOG_FILE);
    let checksum_path = args.out_dir.join(SHA256_FILE);
    fs::write(&catalog_path, &published.catalog_bytes)
        .with_context(|| format!("failed to write {}", catalog_path.display()))?;
    fs::write(&checksum_path, &published.sha256_file_bytes)
        .with_context(|| format!("failed to write {}", checksum_path.display()))?;

    if let Some(evidence_path) = args.evidence_out {
        if let Some(parent) = evidence_path.parent() {
            fs::create_dir_all(parent).with_context(|| {
                format!("failed to create evidence directory {}", parent.display())
            })?;
        }
        let evidence = serde_json::to_vec_pretty(&published.run_evidence)
            .context("failed to encode evidence")?;
        fs::write(&evidence_path, evidence)
            .with_context(|| format!("failed to write {}", evidence_path.display()))?;
    }

    println!("catalog: {}", catalog_path.display());
    println!("sha256: {}", checksum_path.display());
    println!("hash: {}", published.sha256);
    println!("providers: {}", published.document.providers.len());
    println!("models: {}", published.document.models.len());
    Ok(())
}

struct PublisherArgs {
    input: PathBuf,
    out_dir: PathBuf,
    evidence_out: Option<PathBuf>,
}

impl PublisherArgs {
    fn parse<I>(args: I) -> Result<Self>
    where
        I: IntoIterator<Item = String>,
    {
        let mut input = None;
        let mut out_dir = None;
        let mut evidence_out = None;
        let mut args = args.into_iter();
        while let Some(arg) = args.next() {
            match arg.as_str() {
                "--input" => input = Some(next_path(&mut args, "--input")?),
                "--out-dir" => out_dir = Some(next_path(&mut args, "--out-dir")?),
                "--evidence-out" => evidence_out = Some(next_path(&mut args, "--evidence-out")?),
                "-h" | "--help" => bail!("{}", usage()),
                other => bail!("unknown argument `{other}`\n{}", usage()),
            }
        }

        Ok(Self {
            input: input.context("missing --input")?,
            out_dir: out_dir.context("missing --out-dir")?,
            evidence_out,
        })
    }
}

fn next_path(args: &mut impl Iterator<Item = String>, flag: &str) -> Result<PathBuf> {
    let value = args
        .next()
        .with_context(|| format!("{flag} requires a path"))?;
    Ok(Path::new(&value).to_path_buf())
}

fn usage() -> &'static str {
    "usage: duumbi-model-catalog-publisher --input <publisher.json> --out-dir <dir> [--evidence-out <run-evidence.json>]"
}
