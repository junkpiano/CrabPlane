mod adapters;
mod engine;
mod queue;
mod registry;
mod router;
mod tasks;
mod types;
mod unix_signal;

use std::env;
use std::io::IsTerminal;
use std::sync::{Arc, atomic::AtomicBool};
use std::time::Duration;

use adapters::{cli, discord, telegram};
use engine::{Core, Engine, ResultSink};
use queue::Queue;
use registry::Registry;
use router::PrefixRouter;
use tasks::{EchoTask, OpenAiTask, PingTask, Task};
use unix_signal::install_unix_signal_handlers;
use worker::Pool;

mod worker;

struct LogSink;

impl ResultSink for LogSink {
    fn deliver(&self, job: &types::Job, resp: &types::Response) -> Result<(), String> {
        if resp.text.is_empty() {
            return Ok(());
        }
        eprintln!(
            "INFO job result job_id={} task={} text={}",
            job.id, job.task_name, resp.text
        );
        Ok(())
    }
}

#[derive(Clone, Debug)]
struct Args {
    mode: String, // auto|cli|discord|telegram|daemon
    queue_size: usize,
    shutdown_timeout: Duration,
}

fn main() {
    let args = parse_args();

    let stop = Arc::new(AtomicBool::new(false));
    install_unix_signal_handlers(&stop);

    let conc = env_int("CRABPLANE_CONCURRENCY", 4).max(1) as usize;

    let reg = Arc::new(Registry::new());
    must(reg.register(Arc::new(PingTask::new()) as Arc<dyn Task>));
    must(reg.register(Arc::new(EchoTask::new()) as Arc<dyn Task>));
    must(reg.register(Arc::new(OpenAiTask::new()) as Arc<dyn Task>));

    let q = Arc::new(Queue::new(args.queue_size));
    let (pool, results_rx) = Pool::new(Arc::clone(&reg), Arc::clone(&q), conc);

    let router = Arc::new(PrefixRouter::new());

    let selected = select_mode(&args.mode);
    match selected.as_str() {
        "cli" => {
            let sink = Arc::new(cli::Sink::new());
            let core = Core::new(router, reg, pool, results_rx, Some(sink));
            let eng: Arc<dyn Engine> = core.clone();
            let a = cli::Adapter::new(eng);
            let _ = a.run(&stop);
            graceful_shutdown(&stop, args.shutdown_timeout, &core);
        }
        "discord" => {
            let token = env::var("DISCORD_TOKEN").unwrap_or_default();
            // Create engine first, then attach the Discord adapter as a ResultSink.
            let core = Core::new(router, reg, pool, results_rx, None);
            let eng: Arc<dyn Engine> = core.clone();
            let a = Arc::new(discord::Adapter::new(token, eng));
            core.set_sink(Some(a.clone()));
            let _ = discord::Adapter::run(&*a);
            let _ = discord::Adapter::close(&*a);
            graceful_shutdown(&stop, args.shutdown_timeout, &core);
        }
        "telegram" => {
            let token = env::var("TELEGRAM_BOT_TOKEN").unwrap_or_default();
            // Create engine first, then attach the Telegram adapter as a ResultSink.
            let core = Core::new(router, reg, pool, results_rx, None);
            let eng: Arc<dyn Engine> = core.clone();
            let a = Arc::new(telegram::Adapter::new(token, eng));
            core.set_sink(Some(a.clone()));
            let _ = telegram::Adapter::run(&*a, &stop);
            let _ = telegram::Adapter::close(&*a);
            graceful_shutdown(&stop, args.shutdown_timeout, &core);
        }
        "daemon" => {
            let sink = Arc::new(LogSink);
            let core = Core::new(router, reg, pool, results_rx, Some(sink));
            while !stop.load(std::sync::atomic::Ordering::Relaxed) {
                std::thread::sleep(Duration::from_millis(200));
            }
            graceful_shutdown(&stop, args.shutdown_timeout, &core);
        }
        _ => {
            eprintln!("FATAL invalid mode mode={}", selected);
            std::process::exit(2);
        }
    }
}

fn graceful_shutdown(_stop: &AtomicBool, _timeout: Duration, core: &Arc<Core>) {
    // v0: tasks are simple and workers are cooperative; shutdown is best-effort.
    // The timeout is accepted for CLI parity, but we don't force-terminate threads.
    core.shutdown();
}

fn select_mode(mode: &str) -> String {
    let mut selected = mode.to_string();
    if selected == "auto" {
        if env::var("DISCORD_TOKEN").unwrap_or_default() != "" {
            selected = "discord".to_string();
        } else if env::var("TELEGRAM_BOT_TOKEN").unwrap_or_default() != "" {
            selected = "telegram".to_string();
        } else if std::io::stdin().is_terminal() {
            selected = "cli".to_string();
        } else {
            selected = "daemon".to_string();
        }
    }
    selected
}

fn env_int(key: &str, def: i64) -> i64 {
    match env::var(key) {
        Ok(v) => v.parse::<i64>().unwrap_or(def),
        Err(_) => def,
    }
}

fn must<T>(r: Result<T, String>) -> T {
    match r {
        Ok(v) => v,
        Err(e) => panic!("{e}"),
    }
}

fn parse_args() -> Args {
    let mut mode = "auto".to_string();
    let mut queue_size: usize = 128;
    let mut shutdown_timeout = Duration::from_secs(10);

    let mut it = env::args().skip(1);
    while let Some(a) = it.next() {
        let (k, v) = if let Some(v) = a.strip_prefix("--mode=") {
            ("--mode", Some(v.to_string()))
        } else if let Some(v) = a.strip_prefix("--queue-size=") {
            ("--queue-size", Some(v.to_string()))
        } else if let Some(v) = a.strip_prefix("--shutdown-timeout=") {
            ("--shutdown-timeout", Some(v.to_string()))
        } else if a == "-mode" || a == "--mode" {
            ("--mode", it.next())
        } else if a == "-queue-size" || a == "--queue-size" {
            ("--queue-size", it.next())
        } else if a == "-shutdown-timeout" || a == "--shutdown-timeout" {
            ("--shutdown-timeout", it.next())
        } else if a == "-h" || a == "--help" {
            print_help_and_exit();
        } else {
            eprintln!("WARN unknown arg: {}", a);
            continue;
        };

        match (k, v) {
            ("--mode", Some(v)) => mode = v,
            ("--queue-size", Some(v)) => {
                queue_size = v.parse::<usize>().unwrap_or(queue_size);
            }
            ("--shutdown-timeout", Some(v)) => {
                shutdown_timeout = parse_duration(&v).unwrap_or(shutdown_timeout);
            }
            _ => {}
        }
    }

    Args {
        mode,
        queue_size,
        shutdown_timeout,
    }
}

fn parse_duration(s: &str) -> Option<Duration> {
    let s = s.trim();
    if let Some(v) = s.strip_suffix("ms") {
        return v.trim().parse::<u64>().ok().map(Duration::from_millis);
    }
    if let Some(v) = s.strip_suffix('s') {
        return v.trim().parse::<u64>().ok().map(Duration::from_secs);
    }
    if let Some(v) = s.strip_suffix('m') {
        return v
            .trim()
            .parse::<u64>()
            .ok()
            .map(|mins| Duration::from_secs(mins * 60));
    }
    // If no suffix, treat as seconds.
    s.parse::<u64>().ok().map(Duration::from_secs)
}

fn print_help_and_exit() -> ! {
    println!("clawplane v0 (rust port)");
    println!("  -mode auto|cli|discord|telegram|daemon (default: auto)");
    println!("  -queue-size N (default: 128)");
    println!("  -shutdown-timeout 10s|500ms|1m (default: 10s)");
    std::process::exit(0);
}
