use anyhow::Result;
use clap::Parser;

use dende_rs::modules::logwatcher::events::spawn_job_watcher;
use dende_rs::modules::virustotal::spawn_virustotal_watcher;
use env_logger::Builder;
use log::{info,debug,error};

use dende_rs::args::{Args, load_jobs_from_cli_or_yaml};
use dende_rs::Matcher;
use dende_rs::notifiers::Notifier;

#[tokio::main]
async fn main() -> Result<()> {
    // Get arguments
    let args = Args::parse();

    // Build logger
    let level = match args.verbose {
        0 => log::LevelFilter::Info,
        1 => log::LevelFilter::Debug,
        _ => log::LevelFilter::Trace,
    };
    Builder::from_env(env_logger::Env::default().default_filter_or("info"))
        .filter_level(level)
        .init();
    debug!("Verbosity level: {:?}", level);

    // Build the job list (from YAML or CLI) + optional global Telegram token
    let (mut jobs, telegram_global_token, virustotal_global_token, textbelt_global_token) = load_jobs_from_cli_or_yaml(&args)?;

    // Start each job in a blocking thread; the notifier runs in Tokio
    let mut _thread_handles = Vec::new();
    let mut _vt_tasks: Vec<tokio::task::JoinHandle<()>> = Vec::new();
    for (idx, job) in jobs.drain(..).enumerate() {

        // If job has "path" is search job "log-watcher"
        if let Some(path) = job.path.as_ref() {
            if path.is_dir() || path.is_file() {
                let telegram_token = job.telegram_token.clone().or_else(|| telegram_global_token.clone());
                let textbelt_token = job.textbelt_token.clone().or_else(|| textbelt_global_token.clone());                
                let matcher = Matcher::from_spec(&job.search, &job.regex)?;
                let notifier = Notifier::new(job.to.clone(), telegram_token,textbelt_token)?;
                let handle = spawn_job_watcher(
                    idx,
                    path.clone(),
                    job.recursive,
                    job.read_existing,
                    matcher,
                    notifier,
                );
                _thread_handles.push(handle);
            }
        }

        // If job has "hash" is virustotal checker job "virustotal-watcher"
        if let Some(_) = job.hash.as_ref() {
            if let Some(vt_token) = virustotal_global_token.to_owned() {

                let telegram_token = job.telegram_token.clone().or_else(|| telegram_global_token.clone());
                let textbelt_token = job.textbelt_token.clone().or_else(|| textbelt_global_token.clone());
                let notifier = Notifier::new(job.to.clone(), telegram_token, textbelt_token)?;

                if let Some(hashes) = job.hash.clone() {
                    let handle = tokio::spawn(async move {
                        if let Err(e) = spawn_virustotal_watcher(
                            &vt_token, 
                            hashes,
                            notifier
                        ).await {
                            error!("[virustotal] scheduler error: {e}");
                        }
                    });
                    _vt_tasks.push(handle);
                }
            }
        }
    }

    info!("dende-rs: ready. Press Ctrl+C to quit..");
    let _ = tokio::signal::ctrl_c().await;
    info!("Shutdown requested. Bye!");
    Ok(())
}