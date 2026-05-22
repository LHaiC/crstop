use anyhow::{Context, Result};
use clap::Parser;
use crossterm::{
    event::{self, Event, KeyCode, KeyEventKind},
    execute,
    terminal::{EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode},
};
use crstop::{
    app::status_exit_code,
    client::CrsClient,
    config::{load_settings, mask_key},
    history::TrendHistory,
    model::Snapshot,
    ui::{render_dashboard, render_once_text},
};
use ratatui::{Terminal, backend::CrosstermBackend};
use std::{
    fs, io,
    path::PathBuf,
    time::{Duration, Instant},
};

#[derive(Debug, Parser)]
#[command(name = "crstop", about = "btop-like CRS usage dashboard")]
struct Args {
    #[arg(long)]
    config: Option<PathBuf>,
    #[arg(long, default_value_t = 1.0)]
    refresh: f64,
    #[arg(long)]
    once: bool,
    #[arg(long)]
    fixture: Option<PathBuf>,
    #[arg(long)]
    no_cache: bool,
}

fn main() -> Result<()> {
    let args = Args::parse();
    let refresh = normalize_refresh_secs(args.refresh);

    if let Some(path) = &args.fixture {
        let snapshot: Snapshot = serde_json::from_str(
            &fs::read_to_string(path).with_context(|| format!("read {}", path.display()))?,
        )?;
        println!("{}", render_once_text(&snapshot, refresh, 100));
        return Ok(());
    }

    let settings = load_settings(args.config)?;
    let client = CrsClient::new(settings.root_url.clone());

    if args.once {
        let snapshot = client.snapshot(&settings, args.no_cache)?;
        let code = status_exit_code(&crstop::app::build_status(&snapshot).level);
        println!("{}", render_once_text(&snapshot, refresh, 100));
        std::process::exit(code);
    }

    run_tui(client, settings, refresh, args.no_cache)
}

fn run_tui(
    client: CrsClient,
    settings: crstop::config::Settings,
    refresh: f64,
    no_cache: bool,
) -> Result<()> {
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;
    let masked = mask_key(&settings.api_key);
    let mut snapshot = client.snapshot(&settings, no_cache).ok();
    let mut last_refresh = Instant::now();
    let mut trend = TrendHistory::new(1_800);
    if let Some(snap) = &snapshot {
        trend.push_snapshot(snap, last_refresh);
    }
    let mut detailed_daily = true;
    let mut detailed_monthly = false;

    let result = loop {
        terminal.draw(|frame| {
            if let Some(snap) = &snapshot {
                render_dashboard(
                    frame,
                    snap,
                    refresh,
                    &masked,
                    detailed_daily,
                    detailed_monthly,
                    &trend,
                );
            }
        })?;

        let timeout = Duration::from_millis(100);
        if event::poll(timeout)?
            && let Event::Key(key) = event::read()?
            && key.kind == KeyEventKind::Press
        {
            match key.code {
                KeyCode::Char('q') | KeyCode::Esc => break Ok(()),
                KeyCode::Char('c') if key.modifiers.contains(event::KeyModifiers::CONTROL) => {
                    break Ok(());
                }
                KeyCode::Char('r') => {
                    let now = Instant::now();
                    let (next_snapshot, fresh) =
                        refresh_snapshot(&client, &settings, no_cache, snapshot);
                    snapshot = next_snapshot;
                    if fresh && let Some(snap) = &snapshot {
                        trend.push_snapshot(snap, now);
                    }
                    last_refresh = now;
                }
                KeyCode::Char('d') => detailed_daily = !detailed_daily,
                KeyCode::Char('m') => detailed_monthly = !detailed_monthly,
                _ => {}
            }
        }

        if last_refresh.elapsed() >= Duration::from_secs_f64(refresh) {
            let now = Instant::now();
            let (next_snapshot, fresh) = refresh_snapshot(&client, &settings, no_cache, snapshot);
            snapshot = next_snapshot;
            if fresh && let Some(snap) = &snapshot {
                trend.push_snapshot(snap, now);
            }
            last_refresh = now;
        }
    };

    disable_raw_mode()?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen)?;
    terminal.show_cursor()?;
    result
}

fn normalize_refresh_secs(value: f64) -> f64 {
    if value.is_finite() && value > 0.0 {
        value.max(0.1)
    } else {
        1.0
    }
}

fn refresh_snapshot(
    client: &CrsClient,
    settings: &crstop::config::Settings,
    no_cache: bool,
    previous: Option<Snapshot>,
) -> (Option<Snapshot>, bool) {
    match client.snapshot(settings, no_cache) {
        Ok(snap) => (Some(snap), true),
        Err(err) => (
            previous.map(|mut snap| {
                snap.last_error = Some(err.to_string());
                snap
            }),
            false,
        ),
    }
}
