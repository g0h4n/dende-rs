<hr />

- [Help](#help)
  - [Help usage](#help-usage)
  - [Watchers](#watchers)
  - [Notifiers (sinks)](#notifiers-sinks)
- [Quick usage](#quick-usage)
  - [Compilation (Makefile, cargo, docker)](#compilation)
  - [Quick start with Telegram](#quick-start-wth-telegram)
- [YAML example](#yaml-example)
- [How to add a new notifier?](#how-to-add-a-new-notifier)

<hr />

## Help

### Help Usage

```bash
dende-rs monitors files or directories for patterns (string or regex) and sends instant Telegram notifications on matches.
It can also run a dedicated (VT watch) job that polls VirusTotal for your payload’s hash and alerts you the moment it’s published.
Configure it via CLI or YAML to run multiple jobs in parallel.

Usage: dende-rs [OPTIONS]

Options:
  -P, --path <PATH>
          Path to watch (file or folder, single-job CLI mode)
  -S, --search <SEARCH>
          Literal term to search for (single-job CLI mode)
  -R, --regex <REGEX>
          Regular expression (Rust regex, single-job CLI mode)
  -T, --to <TO>
          Recipients: plain text = console tag, 'tg:<CHAT_ID>' = Telegram (single-job CLI mode)
      --recursive
          Watch subdirectories (single-job CLI mode)
      --read-existing
          On startup, read existing files from the beginning (single-job CLI mode)
      --telegram-token <TELEGRAM_TOKEN>
          Telegram bot token (or ENV TELEGRAM_BOT_TOKEN) (single-job CLI mode) [env: TELEGRAM_BOT_TOKEN=]
  -H, --hash <HASH>
          SHA-256 hash of the payload to monitor on VirusTotal. Checks whether the binary has been published. (single-job CLI mode)
      --virustotal-token <VIRUSTOTAL_TOKEN>
          Virustotal API token to check the payload hashes is published or not (or ENV VIRUSTOTAL_TOKEN) (single-job CLI mode) [env: VIRUSTOTAL_TOKEN=]
      --textbelt-token <TEXTBELT_TOKEN>
          Textbelt API token to send SMS [env: TEXTBELT_TOKEN=]
  -C, --config <CONFIG>
          YAML configuration file (multi-jobs)
  -v...
          Verbosity (-v, -vv, -vvv)
      --only-message
          Send only the matched message
  -h, --help
          Print help
```

### Watchers

> All watchers sends an alert immediately to the configured sinks.

- [x] Log watcher
  - **Description:** Tail a single file or a directory (optionally recursive) and match each line via a literal string or regex.
  - **Limitation:** N/A

- [x] Payload watcher on virustotal
  - **Description:** Periodically check whether one or more SHA-256 values of your payloads have been published on VirusTotal.
  - **Limitation:** Free API limitation 1 request for 1 hash every ~216s

### Notifiers (sinks)

> The notification layer is modular. Built-ins: console and Telegram. It’s easy to add more sinks (e.g., Slack, email, webhooks, SMS) without touching the watchers.

- [x] Console
  - **Description:** Prints alerts to STDOUT with a tag, no external dependencies. Great for local dev, systemd journaling, or piping into other tools.
  - **Command line/YAML parameter:** `"console"`

- [x] Telegram
  - **Description:** Sends alerts via a Telegram bot to a user.
  - **Command line/YAML parameter:** `"tg:ID"` (e.g., `"tg:123456789"`)

- [ ] Email
  - **Description:** TODO (planned: SMTP/API-based email alerts with subject templating and batching).
  - **Command line/YAML parameter:** `"email:ID"` (e.g., `"email:ops@example.com"`)

- [x] SMS
  - **Description:** Send alerts via SMS using Textbelt API
  - **Command line/YAML parameter:** `"sms:ID"` (e.g., "`sms:+33612345678`")

## Quick usage

### Compilation

This project can be compiled directly from make command like:

```bash
# Compile it for your current system
make release
# Compile it for Windows
make windows
```

Or using docker like below:

```bash
docker build --rm -t dende-rs .

# Then
docker run --rm -v $PWD:/usr/src/dende-rs dende-rs help
docker run --rm -v $PWD:/usr/src/dende-rs dende-rs release
docker run --rm -v $PWD:/usr/src/dende-rs dende-rs windows
docker run --rm -v $PWD:/usr/src/dende-rs dende-rs linux
```

### Quick start wth Telegram

1. Download [Rust](https://www.rust-lang.org/tools/install).
2. Create a new bot using [@Botfather](https://core.telegram.org/bots/tutorial#obtain-your-bot-token) to get a token in the format `0123456789:XXXXxXXxxxXxX3x3x-3X-XxxxX3XXXXxx3X`.
3. Put these token into YAML file or directly as CLI argument.
4. Obtain your Telegram UserId for user who would like to contact, by sending `/start` to [@userinfo](https://telegram.me/userinfobot).
5. Put these UserId YAML file like `tg:UserId` or directly as CLI argument.
6. Please have each Telegram user send /start to your bot (bots can’t initiate DMs).
7. Make sure that your [Rust](https://www.rust-lang.org/tools/install) compiler is up to date:


```bash
$ rustup update nightly
$ rustup override set nightly
```

8. Compilation:

```bash
cargo build --release
```

9. Usage:

```bash
# YAML multi-job mode
target/debug/dende-rs --config ./config_example.yaml

# Single-job CLI from string search
target/release/dende-rs -- \
  -P /var/log/myapp \
  -S ERROR \
  -T console -T tg:123456789 \
  --recursive

# Single-job CLI from regex search
target/release/dende-rs -P /var/log/myapp/access.log -R "^SUCCESS.*" -T tg:123456789
```

## YAML example

```yaml
# config.yaml

# Notifiers
telegram_token: "1234567890:FIXME-FIXME"  # Telegram API token for you bot
# textbelt_token: "token" # Textbelt API token for SMS
# fixme_token: "token"

# Applications
virustotal_token: "FIXME"

jobs:
  # Job 1 (log-watcher)
  - path: "/tmp/logs/apache2/"            # Path of main folder where to search
    search: "ERROR"                       # Using simple string to search
    recursive: true                       # Recurse other folders inside the main folder
    read_existing: false                  # Only read new files
    only_message: false                   # Print all info in the notificaiton
    to: ["console:log"]                   # Only on console 

  # Job 2 (log-watcher)
  - path: "/tmp/logs/ssh/"                # Path of main folder where to search
    regex: "^password=.*"                 # Using simple string to search
    recursive: true                       # Recurse other folders inside the main folder
    read_existing: true                   # Only read new files
    only_message: true                    # Print only the message in the notification
    to: ["tg:FIXME", "sms:+33601020304"]  # Telegram and SMS by textbelt API
  
  # Job 3 (log-watcher)
  - path: "/tmp/logs/nginx/access.log"    # Or path of one file
    regex: '^SUCCESS.*'                   # Using regex
    to: ["console:log", "tg:FIXME", "sms:+33612345678"]       # Console + Telegram + SMS

  # Job 4 (virustotal-watcher) (Check if your payload will be publish on virustotal and notify you)
  - hash: [ 
            "61c0810a23580cf492a6ba4f7654566108331e7a4134c968c2d6a05261b2d8a1", # SHA-256 of your payload
            "11e031526c1e5e177c9fac5be0a3d0383f74ab98399a01adebd42908a3a2fe20", # SHA-256 of your payload
          ]
    to: ["console:log", "tg:FIXME"]       # Console + Telegram 
```

## How to add a new notifier?

1. To add new notifier API, you just need to create new rs file like the following example: `src/notifiers/newnotifier.rs`

```rust
use anyhow::Result;
use log::{info,debug,error};
// other API import

#[derive(Clone)]
pub struct NewNotifierSink {
    bot: Bot,
    chat_id: ChatId,
}

impl std::fmt::Debug for NewNotifierSink {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "FIXME")
    }
}

impl NewNotifierSink {
    pub fn new(token: String, chat_id: i64) -> Self {
        // FIXME create 
    }

    pub async fn send(&self, msg: &str) -> Result<()> {
        info!("Sending notification from FIXME..");
        // FIXME send message
        debug!("Sent by FIXME to FIXME");
        Ok(())
    }
}
```

2. Add `pub mod newnotifier;` inside the rs file: `src/notifiers/mod.rs`
3. Add `NewNotifier(NewNotifierSink),` in the `enum Sink{}` inside the rs file: `src/notifiers/mod.rs`
4. Add `Sink::NewNotifier(s) => s.send(_text).await,` in the `send()` function inside the rs file: `src/notifiers/mod.rs`
5. Add match for your notifier inside in the `new()` function inside the rs file: `src/notifiers/mod.rs`