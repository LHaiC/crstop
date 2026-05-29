use crstop::{history::TrendHistory, model::Snapshot, ui::render_dashboard};
use ratatui::{Terminal, backend::TestBackend};
use std::time::{Duration, Instant};

fn sample_snapshot() -> Snapshot {
    serde_json::from_str(include_str!("fixtures/sample_snapshot.json")).unwrap()
}

fn screen_text(terminal: &Terminal<TestBackend>) -> String {
    terminal
        .backend()
        .buffer()
        .content()
        .iter()
        .map(|cell| cell.symbol())
        .collect()
}

fn screen_grid(terminal: &Terminal<TestBackend>) -> Vec<Vec<String>> {
    let buffer = terminal.backend().buffer();
    buffer
        .content()
        .chunks(buffer.area.width as usize)
        .map(|row| row.iter().map(|cell| cell.symbol().to_string()).collect())
        .collect()
}

fn row_text(row: &[String]) -> String {
    row.iter().map(String::as_str).collect()
}

fn table_header_after_title(grid: &[Vec<String>], title: &str) -> String {
    let title_y = grid
        .iter()
        .position(|row| row_text(row).contains(title))
        .unwrap();
    row_text(&grid[title_y + 1])
}

fn find_sequence(row: &[String], needle: &[&str]) -> Option<usize> {
    row.windows(needle.len()).position(|window| {
        window
            .iter()
            .zip(needle)
            .all(|(cell, expected)| cell == expected)
    })
}

fn live_trend_outer_height(grid: &[Vec<String>]) -> usize {
    let (top_y, _) = grid
        .iter()
        .enumerate()
        .find_map(|(y, row)| {
            find_sequence(row, &["L", "I", "V", "E", " ", "T", "R", "E", "N", "D"]).map(|x| (y, x))
        })
        .unwrap();
    let bottom_y = grid
        .iter()
        .enumerate()
        .skip(top_y + 1)
        .find(|(_, row)| row.first().is_some_and(|cell| cell == "╰"))
        .map(|(y, _)| y)
        .unwrap();
    bottom_y - top_y + 1
}

#[test]
fn dashboard_renders_live_trend_panel_when_history_has_deltas() {
    let mut first = sample_snapshot();
    let mut second = first.clone();
    let start = Instant::now();
    second.user.total.requests += 8;
    second.user.total.all_tokens += 16_000;
    second.user.total.cost += 0.42;

    let mut history = TrendHistory::new(16);
    history.push_snapshot(&first, start);
    history.push_snapshot(&second, start + Duration::from_secs(5));
    history.push_snapshot(&second, start + Duration::from_secs(31));

    let backend = TestBackend::new(120, 32);
    let mut terminal = Terminal::new(backend).unwrap();
    terminal
        .draw(|frame| render_dashboard(frame, &second, 5.0, "masked-key", true, false, &history))
        .unwrap();

    let text = screen_text(&terminal);
    assert!(text.contains("LIVE TREND"));
    assert!(text.contains("REQ 30s"));
    assert!(text.contains("+8"));
    assert!(text.contains("TOKENS 30s"));
    assert!(text.contains("+16.0K"));
    assert!(text.contains("COST 30s"));
    assert!(text.contains("+$0.42"));

    first.user.total.requests += 100;
    assert!(text.contains("+8"));
}

#[test]
fn live_trend_places_latest_bar_at_right_edge() {
    let first = sample_snapshot();
    let mut second = first.clone();
    let start = Instant::now();
    second.user.total.requests += 8;

    let mut history = TrendHistory::new(16);
    history.push_snapshot(&first, start);
    history.push_snapshot(&second, start + Duration::from_millis(500));
    history.push_snapshot(&second, start + Duration::from_secs(31));

    let backend = TestBackend::new(120, 32);
    let mut terminal = Terminal::new(backend).unwrap();
    terminal
        .draw(|frame| render_dashboard(frame, &second, 0.5, "masked-key", true, false, &history))
        .unwrap();

    let grid = screen_grid(&terminal);
    let (title_y, title_x) = grid
        .iter()
        .enumerate()
        .find_map(|(y, row)| {
            find_sequence(row, &["R", "E", "Q", " ", "3", "0", "s"]).map(|x| (y, x))
        })
        .unwrap();
    let right_border_x = grid[title_y][title_x..]
        .iter()
        .position(|cell| cell == "╮")
        .map(|offset| title_x + offset)
        .unwrap();
    let right_inner_x = right_border_x - 1;

    assert_ne!(grid[title_y + 1][right_inner_x], " ");
    assert_ne!(grid[title_y + 2][right_inner_x], " ");
    assert_ne!(grid[title_y + 3][right_inner_x], " ");
}

#[test]
fn live_trend_height_grows_with_terminal_height() {
    let first = sample_snapshot();
    let mut second = first.clone();
    let start = Instant::now();
    second.user.total.requests += 8;

    let mut history = TrendHistory::new(16);
    history.push_snapshot(&first, start);
    history.push_snapshot(&second, start + Duration::from_secs(1));

    let small_backend = TestBackend::new(120, 32);
    let mut small = Terminal::new(small_backend).unwrap();
    small
        .draw(|frame| render_dashboard(frame, &second, 1.0, "masked-key", true, false, &history))
        .unwrap();

    let tall_backend = TestBackend::new(120, 60);
    let mut tall = Terminal::new(tall_backend).unwrap();
    tall.draw(|frame| render_dashboard(frame, &second, 1.0, "masked-key", true, false, &history))
        .unwrap();

    let small_height = live_trend_outer_height(&screen_grid(&small));
    let tall_height = live_trend_outer_height(&screen_grid(&tall));

    assert!(small_height >= 7);
    assert!(tall_height > small_height);
}

#[test]
fn live_trend_uses_continuous_thin_bars_without_partial_glyphs() {
    let mut snapshot = sample_snapshot();
    let start = Instant::now();
    let mut history = TrendHistory::new(16);
    history.push_snapshot(&snapshot, start);

    snapshot.user.total.requests += 8;
    history.push_snapshot(&snapshot, start + Duration::from_secs(2));
    snapshot.user.total.requests += 3;
    history.push_snapshot(&snapshot, start + Duration::from_secs(32));
    history.push_snapshot(&snapshot, start + Duration::from_secs(62));

    let backend = TestBackend::new(120, 40);
    let mut terminal = Terminal::new(backend).unwrap();
    terminal
        .draw(|frame| render_dashboard(frame, &snapshot, 1.0, "masked-key", true, false, &history))
        .unwrap();

    let text = screen_text(&terminal);
    assert!(text.contains("▏"));
    assert!(!text.contains("╵"));
    assert!(!text.contains("┃"));
    assert!(!text.contains("█"));
}

#[test]
fn live_trend_clears_stale_tall_bar_cells_between_frames() {
    let mut snapshot = sample_snapshot();
    let start = Instant::now();
    let mut first_history = TrendHistory::new(16);
    first_history.push_snapshot(&snapshot, start);
    snapshot.user.total.requests += 8;
    first_history.push_snapshot(&snapshot, start + Duration::from_secs(2));
    first_history.push_snapshot(&snapshot, start + Duration::from_secs(31));

    let backend = TestBackend::new(120, 40);
    let mut terminal = Terminal::new(backend).unwrap();
    terminal
        .draw(|frame| {
            render_dashboard(
                frame,
                &snapshot,
                1.0,
                "masked-key",
                true,
                false,
                &first_history,
            )
        })
        .unwrap();

    let mut second_history = first_history.clone();
    snapshot.user.total.requests += 3;
    second_history.push_snapshot(&snapshot, start + Duration::from_secs(32));
    second_history.push_snapshot(&snapshot, start + Duration::from_secs(62));
    terminal
        .draw(|frame| {
            render_dashboard(
                frame,
                &snapshot,
                1.0,
                "masked-key",
                true,
                false,
                &second_history,
            )
        })
        .unwrap();

    let grid = screen_grid(&terminal);
    let (title_y, title_x) = grid
        .iter()
        .enumerate()
        .find_map(|(y, row)| {
            find_sequence(row, &["R", "E", "Q", " ", "3", "0", "s"]).map(|x| (y, x))
        })
        .unwrap();
    let right_border_x = grid[title_y][title_x..]
        .iter()
        .position(|cell| cell == "╮")
        .map(|offset| title_x + offset)
        .unwrap();
    let right_inner_x = right_border_x - 1;

    assert_eq!(grid[title_y + 1][right_inner_x], " ");
    assert_eq!(grid[title_y + 2][right_inner_x], " ");
    assert_eq!(grid[title_y + 3][right_inner_x], "▏");
}

#[test]
fn wide_daily_table_keeps_full_detail_columns() {
    let snapshot = sample_snapshot();
    let history = TrendHistory::new(16);
    let backend = TestBackend::new(140, 32);
    let mut terminal = Terminal::new(backend).unwrap();

    terminal
        .draw(|frame| render_dashboard(frame, &snapshot, 1.0, "masked-key", true, false, &history))
        .unwrap();

    let header = table_header_after_title(&screen_grid(&terminal), "TODAY BY MODEL");
    assert!(header.contains("INPUT"));
    assert!(header.contains("OUTPUT"));
    assert!(header.contains("CACHE"));
    assert!(header.contains("ALL"));
}

#[test]
fn medium_daily_table_hides_cache_before_input_and_output() {
    let snapshot = sample_snapshot();
    let history = TrendHistory::new(16);
    let backend = TestBackend::new(116, 32);
    let mut terminal = Terminal::new(backend).unwrap();

    terminal
        .draw(|frame| render_dashboard(frame, &snapshot, 1.0, "masked-key", true, false, &history))
        .unwrap();

    let header = table_header_after_title(&screen_grid(&terminal), "TODAY BY MODEL");
    assert!(header.contains("INPUT"));
    assert!(header.contains("OUTPUT"));
    assert!(!header.contains("CACHE"));
    assert!(header.contains("ALL"));
}

#[test]
fn narrow_daily_table_keeps_core_columns_only() {
    let snapshot = sample_snapshot();
    let history = TrendHistory::new(16);
    let backend = TestBackend::new(86, 32);
    let mut terminal = Terminal::new(backend).unwrap();

    terminal
        .draw(|frame| render_dashboard(frame, &snapshot, 1.0, "masked-key", true, false, &history))
        .unwrap();

    let header = table_header_after_title(&screen_grid(&terminal), "TODAY BY MODEL");
    assert!(header.contains("MODEL"));
    assert!(header.contains("REQ"));
    assert!(!header.contains("INPUT"));
    assert!(!header.contains("OUTPUT"));
    assert!(!header.contains("CACHE"));
    assert!(header.contains("ALL"));
    assert!(header.contains("COST"));
}
