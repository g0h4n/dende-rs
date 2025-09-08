use anyhow::Result;
use tokio::task::JoinHandle;
use std::time::Duration;

pub mod telegram;
pub mod console;
pub mod textbelt;
// pub mod newnotifier;

use console::ConsoleSink;
use telegram::TelegramSink;
use textbelt::TextbeltSink;
use log::{trace,error};


/// Aggregates all selected sinks and dispatches notifications to them.
pub struct Notifier {
    tx: tokio::sync::mpsc::UnboundedSender<NotifyEvent>,
    #[allow(dead_code)]
    task: JoinHandle<()>,
}

#[derive(Clone, Debug)]
pub struct NotifyEvent {
    pub msg: String,
}

/// Concrete sink types we support. Add new variants as you add files.
enum Sink {
    Console(ConsoleSink),
    Telegram(TelegramSink),
    Textbelt(TextbeltSink),
    // NewNotifier(NewNotifierSink),
}

impl Sink {
    async fn send(&self, _msg: &str) -> Result<()> {
        match self {
            Sink::Console(s) => s.send(_msg).await,
            Sink::Telegram(s) => s.send(_msg).await,
            Sink::Textbelt(s) => s.send(_msg).await,
            // // Easy to add another notifier here
            // Sink::NewNotifier(s) => s.send(_text).await,
        }
    }
}

impl Notifier {
    pub fn new(
        to_raw: Vec<String>,
        telegram_token: Option<String>,
        textbelt_token: Option<String>,
    ) -> Result<Self> {

        let mut sinks: Vec<Sink> = Vec::new();

        for to in to_raw {
            let to = to.trim();

            match to.split_once(':') {
                
                // Telegram notifier
                Some(("tg", id)) => {
                    match id.parse::<i64>() {
                        Ok(id) => {
                            if let Some(token) = telegram_token.clone() {
                                sinks.push(Sink::Telegram(TelegramSink::new(token, id)));
                            } else {
                                error!("Skipping Telegram dest {id}: no token provided");
                                continue;
                            }
                        }
                        Err(e) => {
                            error!("Invalid Telegram UserId({id}): {e}");
                            continue;
                        },
                    }
                }

                // Email notifier
                Some(("email", _addr)) => {
                    error!("Email notification has not been yet implemented.");
                    continue;
                },
                /*
                // SMS notifier
                Some(("sms", _num)) => {
                    error!("SMS notification has not been yet implemented.");
                    continue;
                }
                */
                Some(("sms", _num)) => {
                    if let Some(token) = textbelt_token.clone(){
                        sinks.push(Sink::Textbelt(TextbeltSink::new(token, _num.to_string())));
                    } else {
                        error!("Skipping SMS dest {_num}: no Textbelt API key provided");
                        continue;
                    }
                }
                // Console notifier
                Some(("console", tag)) => {
                    sinks.push(Sink::Console(ConsoleSink::new(tag.to_string())));
                    continue;
                }

                // Unknown notifier
                Some((_scheme, _rest)) => {
                    error!("{_scheme} unknown!");
                    continue;
                }

                None => {
                    continue;
                }
            }

        }

        let (tx, mut rx) = tokio::sync::mpsc::unbounded_channel::<NotifyEvent>();

        let task = tokio::spawn(async move {
            while let Some(ev) = rx.recv().await {
                for sink in &sinks {
                    let mut delay = Duration::from_millis(400);
                    for attempt in 1..=3 {
                        let res = sink.send(&ev.msg).await;
                        match res {
                            Ok(_) => {
                                trace!("notifier sink well work, notification sent!");
                                break;
                            }
                            Err(e) => {
                                if attempt == 3 {
                                    error!("send failed after {attempt} attempts: {e}");
                                    break;
                                }
                                error!("notifier sink error, send failed (attempt {attempt}/3): {e} - retry in {} ms",delay.as_millis());
                                tokio::time::sleep(delay).await;
                                delay = std::cmp::min(delay * 2, Duration::from_secs(5));
                            }
                        }
                    }
                }
            }
        });

        Ok(Self { tx, task })
    }

    /// Queue a notification event for processing by the async task.
    pub fn notify(&self, msg: &str) {
        let _ = self.tx.send(NotifyEvent {
            msg: msg.to_string(),
        });
    }
}