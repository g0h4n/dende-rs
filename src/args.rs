use anyhow::{Result, Context};
use clap::{ArgAction, Parser};
use serde::Deserialize;
use std::path::PathBuf;

/// CLI arguments for single-job mode or --config YAML multi-job mode.
#[derive(Parser, Debug)]
#[command(name = "dende-rs (デンデ, Dende)")]
#[command(author = "@g0h4n_0")]
#[command(about = "dende-rs monitors files or directories for patterns (string or regex) and sends instant Telegram notifications on matches.\nIt can also run a dedicated (VT watch) job that polls VirusTotal for your payload’s hash and alerts you the moment it’s published.\nConfigure it via CLI or YAML to run multiple jobs in parallel.")]
pub struct Args {
    /// Path to watch (file or folder, single-job CLI mode)
    #[arg(short = 'P', long = "path")]
    pub path: Option<PathBuf>,

    /// Literal term to search for (single-job CLI mode)
    #[arg(short = 'S', long = "search")]
    pub search: Option<String>,

    /// Regular expression (Rust regex, single-job CLI mode)
    #[arg(short = 'R', long = "regex")]
    pub regex: Option<String>,

    /// Recipients: plain text = console tag, 'tg:<CHAT_ID>' = Telegram (single-job CLI mode)
    #[arg(short = 'T', long = "to", value_delimiter = ',')]
    pub to: Vec<String>,

    /// Watch subdirectories (single-job CLI mode)
    #[arg(long = "recursive", default_value_t = false)]
    pub recursive: bool,

    /// On startup, read existing files from the beginning (single-job CLI mode)
    #[arg(long = "read-existing", default_value_t = true)]
    pub read_existing: bool,

    /// Telegram bot token (or ENV TELEGRAM_BOT_TOKEN) (single-job CLI mode)
    #[arg(long = "telegram-token", env = "TELEGRAM_BOT_TOKEN")]
    pub telegram_token: Option<String>,

    /// SHA-256 hash of the payload to monitor on VirusTotal. Checks whether the binary has been published. (single-job CLI mode)
    #[arg(short = 'H', long = "hash")]
    pub hash: Option<Vec<String>>,

    /// Virustotal API token to check the payload hashes is published or not (or ENV VIRUSTOTAL_TOKEN) (single-job CLI mode)
    #[arg(long = "virustotal-token", env = "VIRUSTOTAL_TOKEN")]
    pub virustotal_token: Option<String>,
    /// Textbelt API token to send SMS
    #[arg(long = "textbelt-token", env = "TEXTBELT_TOKEN")]
    pub textbelt_token: Option<String>,
    /// YAML configuration file (multi-jobs)
    #[arg(short = 'C', long = "config")]
    pub config: Option<PathBuf>,

    /// Verbosity (-v, -vv, -vvv)
    #[arg(short = 'v', action = ArgAction::Count)]
    pub verbose: u8,
}

#[derive(Debug, Deserialize, Clone)]
pub struct JobSpec {
    pub path: Option<PathBuf>,
    #[serde(default)]
    pub search: Option<String>,
    #[serde(default)]
    pub regex: Option<String>,
    #[serde(default)]
    pub to: Vec<String>,
    #[serde(default = "default_false")]
    pub recursive: bool,
    #[serde(default = "default_true")]
    pub read_existing: bool,
    #[serde(default)]
    pub telegram_token: Option<String>,
    #[serde(default)]
    pub hash: Option<Vec<String>>,
    #[serde(default)]
    pub virustotal_token: Option<String>,
    #[serde(default)]
    pub textbelt_token: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct ConfigFile {
    #[serde(default)]
    pub telegram_token: Option<String>,
    #[serde(default)]
    pub virustotal_token: Option<String>,
    #[serde(default)]
    pub textbelt_token: Option<String>,
    pub jobs: Vec<JobSpec>,
}

fn default_true() -> bool { true }
fn default_false() -> bool { false }

/// Load args from CLI or from YAML file
pub fn load_jobs_from_cli_or_yaml(args: &Args)
 -> Result<(Vec<JobSpec>, Option<String>, Option<String>, Option<String>)> {
    if let Some(cfg_path) = args.config.as_ref() {
        let text = std::fs::read_to_string(cfg_path)
            .with_context(|| format!("Reading config file: {}", cfg_path.display()))?;
        let cfg: ConfigFile = serde_yaml::from_str(&text)
            .with_context(|| "Parsing YAML configuration")?;

        if cfg.jobs.is_empty() {
            anyhow::bail!("YAML file contains no jobs.");
        }

        for (i, j) in cfg.jobs.iter().enumerate() {
            let is_vt = j
                .hash
                .as_ref()
                .map(|v| v.iter().any(|s| !s.trim().is_empty()))
                .unwrap_or(false);
            let has_path = j.path.is_some();

            // Path XOR Hash
            if !is_vt && !has_path {
                anyhow::bail!("Job #{i}: specify either 'path' (file/dir) or 'hash' (VirusTotal).");
            }
            if is_vt && has_path {
                anyhow::bail!("Job #{i}: choose only one of 'path' or 'hash', not both.");
            }

            // Common: need one recipient
            if j.to.is_empty() {
                anyhow::bail!("Job #{i}: specify at least one recipient in 'to'.");
            }

            if is_vt {
                // VT job: no exigence search/regex/path
                continue;
            } else {
                // File/dir job
                let path = j.path.as_ref().unwrap();
                if !path.is_dir() && !path.is_file() {
                    anyhow::bail!(
                        "Job #{i}: non-existent file or directory on path: {}",
                        path.display()
                    );
                }
                if j.search.is_none() && j.regex.is_none() {
                    anyhow::bail!("Job #{i}: specify 'search' or 'regex' for file/dir jobs.");
                }
            }
        }

        return Ok((cfg.jobs, cfg.telegram_token, cfg.virustotal_token, cfg.textbelt_token));
    }

    // --- CLI (one job only) ---
    // --path + (--search|--regex), or --hash
    if let Some(h) = args.hash.as_ref() {
        if args.to.is_empty() {
            anyhow::bail!("Specify at least one recipient via -T/--to.");
        }
        let job = JobSpec {
            path: None,
            search: None,
            regex: None,
            to: args.to.clone(),
            recursive: false,
            read_existing: false,
            telegram_token: args.telegram_token.clone(),
            hash: Some(h.clone()),
            virustotal_token: args.virustotal_token.clone(),
            textbelt_token: args.textbelt_token.clone(),
        };
        Ok((vec![job], None, None, None))
    } else {
        let path = args.path.as_ref()
            .ok_or_else(|| anyhow::anyhow!("--path required in CLI mode (or use --config)"))?;
        if !path.is_dir() && !path.is_file() {
            anyhow::bail!("--path must be an existing file or directory");
        }
        if args.search.is_none() && args.regex.is_none() {
            anyhow::bail!("Specify --search or --regex (or use --config).");
        }
        if args.to.is_empty() {
            anyhow::bail!("Specify at least one recipient via -T/--to.");
        }
        let job = JobSpec {
            path: Some(path.clone()),
            search: args.search.clone(),
            regex: args.regex.clone(),
            to: args.to.clone(),
            recursive: args.recursive,
            read_existing: args.read_existing,
            telegram_token: args.telegram_token.clone(),
            hash: None,
            virustotal_token: args.virustotal_token.clone(),
            textbelt_token: args.textbelt_token.clone(),
        };
        Ok((vec![job], None, None, None))
    }
}