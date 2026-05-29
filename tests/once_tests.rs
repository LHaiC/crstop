use crstop::model::Snapshot;
use crstop::ui::render_once_text;

#[test]
fn once_text_contains_btop_like_sections() {
    let fixture = include_str!("fixtures/sample_snapshot.json");
    let snapshot: Snapshot = serde_json::from_str(fixture).unwrap();
    let text = render_once_text(&snapshot, 5.0, 100);

    assert!(text.contains("crstop"));
    assert!(text.contains("TOTAL"));
    assert!(text.contains("LIMITS"));
    assert!(text.contains("TODAY BY MODEL"));
    assert!(text.contains("MONTH BY MODEL"));
    assert!(text.contains("pool limits: not visible without admin token"));
    assert!(text.contains("gpt-5.5"));
    assert!(!text.contains("LIVE TREND"));
}

#[test]
fn once_text_uses_colored_dashboard_accents() {
    let fixture = include_str!("fixtures/sample_snapshot.json");
    let snapshot: Snapshot = serde_json::from_str(fixture).unwrap();
    let text = render_once_text(&snapshot, 5.0, 100);

    assert!(text.contains("\u{1b}[38;5;45m"));
    assert!(text.contains("\u{1b}[38;5;82mOK"));
    assert!(text.contains("╭"));
    assert!(text.contains("╰"));
    assert!(text.contains("▰"));
}

#[test]
fn startup_error_dashboard_explains_initial_snapshot_failure() {
    let backend = ratatui::backend::TestBackend::new(80, 10);
    let mut terminal = ratatui::Terminal::new(backend).unwrap();

    terminal
        .draw(|frame| {
            crstop::ui::render_error_dashboard(
                frame,
                "POST /apiStats/api/get-key-id: http status 400",
            )
        })
        .unwrap();

    let text: String = terminal
        .backend()
        .buffer()
        .content()
        .iter()
        .map(|cell| cell.symbol())
        .collect();
    assert!(text.contains("crstop"));
    assert!(text.contains("startup error"));
    assert!(text.contains("get-key-id"));
}
