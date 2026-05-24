use crate::{
    app::build_status,
    format::{comma, compact_number, display_cost, dollars, sorted_models},
    history::TrendHistory,
    model::{ModelStat, Snapshot},
};
use ratatui::{
    Frame,
    layout::{Constraint, Direction, Layout, Margin, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, BorderType, Borders, Cell, Paragraph, Row, Table},
};
use std::time::Duration;

const ANSI_RESET: &str = "\x1b[0m";
const ANSI_CYAN: &str = "\x1b[38;5;45m";
const ANSI_GREEN: &str = "\x1b[38;5;82m";
const ANSI_YELLOW: &str = "\x1b[38;5;220m";
const ANSI_RED: &str = "\x1b[38;5;196m";
const ANSI_PURPLE: &str = "\x1b[38;5;141m";
const ANSI_BLUE: &str = "\x1b[38;5;39m";
const ANSI_DIM: &str = "\x1b[38;5;244m";
const ANSI_BOLD: &str = "\x1b[1m";
const TREND_BUCKET_SECS: u64 = 30;
const TREND_BAR_SYMBOL: &str = "▏";

pub fn render_once_text(snapshot: &Snapshot, refresh_secs: f64, width: u16) -> String {
    let status = build_status(snapshot);
    let mut out = String::new();
    let status_color = ansi_status_color(&status.level);
    let line_width = width.max(60) as usize;
    out.push_str(&format!(
        "{ANSI_CYAN}{ANSI_BOLD}╭─ crstop{ANSI_RESET}  {status_color}▰ {}{ANSI_RESET}  {ANSI_DIM}CRS{ANSI_RESET}: {}  {ANSI_DIM}v{ANSI_RESET}{}  {ANSI_DIM}Redis{ANSI_RESET}: {}  {ANSI_DIM}Refresh{ANSI_RESET}: {}s  {ANSI_DIM}Last{ANSI_RESET}: {}\n",
        status.level,
        snapshot.health.status,
        snapshot.health.version.as_deref().unwrap_or("-"),
        snapshot.health.redis_status.as_deref().unwrap_or("-"),
        format_secs(refresh_secs),
        snapshot.fetched_at
    ));
    out.push_str(
        &format!("{ANSI_CYAN}{}{ANSI_RESET}", "─".repeat(line_width))
            .chars()
            .take(line_width + ANSI_CYAN.len() + ANSI_RESET.len())
            .collect::<String>(),
    );
    out.push_str(&format!(
        "\n{ANSI_CYAN}╰─{ANSI_RESET} {ANSI_DIM}Key{ANSI_RESET}: {}  {ANSI_DIM}Theme{ANSI_RESET}: neon telemetry\n\n",
        if snapshot.user.is_active {
            "active"
        } else {
            "inactive"
        }
    ));
    push_section_title(&mut out, "TOTAL", ANSI_CYAN);
    out.push_str(&format!(
        "  {ANSI_DIM}Requests{ANSI_RESET}      {ANSI_BLUE}{}{ANSI_RESET}\n",
        comma(snapshot.user.total.requests),
    ));
    out.push_str(&format!(
        "  {ANSI_DIM}All Tokens{ANSI_RESET}    {ANSI_CYAN}{}{ANSI_RESET}\n",
        comma(snapshot.user.total.all_tokens)
    ));
    out.push_str(&format!(
        "  {ANSI_DIM}Input{ANSI_RESET}         {}\n",
        comma(snapshot.user.total.input_tokens)
    ));
    out.push_str(&format!(
        "  {ANSI_DIM}Output{ANSI_RESET}        {}\n",
        comma(snapshot.user.total.output_tokens)
    ));
    out.push_str(&format!(
        "  {ANSI_DIM}Cache Read{ANSI_RESET}    {}\n",
        comma(snapshot.user.total.cache_read_tokens)
    ));
    out.push_str(&format!(
        "  {ANSI_DIM}Cost{ANSI_RESET}          {ANSI_GREEN}{}{ANSI_RESET}\n\n",
        display_cost(
            snapshot.user.total.cost,
            &snapshot.user.total.formatted_cost
        )
    ));

    push_section_title(&mut out, "LIMITS", ANSI_PURPLE);
    out.push_str(&format!(
        "  {ANSI_DIM}Daily Cost{ANSI_RESET}    {} / {}  {}\n",
        dollars(snapshot.user.limits.current_daily_cost),
        limit_cost(snapshot.user.limits.daily_cost_limit),
        usage_bar(
            snapshot.user.limits.current_daily_cost,
            snapshot.user.limits.daily_cost_limit
        )
    ));
    out.push_str(&format!(
        "  {ANSI_DIM}Total Cost{ANSI_RESET}    {} / {}  {}\n",
        dollars(snapshot.user.limits.current_total_cost),
        limit_cost(snapshot.user.limits.total_cost_limit),
        usage_bar(
            snapshot.user.limits.current_total_cost,
            snapshot.user.limits.total_cost_limit
        )
    ));
    out.push_str(&format!(
        "  {ANSI_DIM}Concurrency{ANSI_RESET}   {}\n",
        limit_u64(snapshot.user.limits.concurrency_limit)
    ));
    out.push_str(&format!(
        "  {ANSI_DIM}Rate Limit{ANSI_RESET}    {}\n",
        limit_u64(snapshot.user.limits.rate_limit_requests)
    ));
    out.push_str(&format!(
        "  {ANSI_DIM}Pool Limits{ANSI_RESET}   {ANSI_YELLOW}{}{ANSI_RESET}\n\n",
        if snapshot.pool_visible {
            "visible"
        } else {
            "not visible without admin token"
        }
    ));

    push_model_section(&mut out, "TODAY BY MODEL", &snapshot.daily, true);
    out.push('\n');
    push_model_section(&mut out, "MONTH BY MODEL", &snapshot.monthly, false);
    out.push('\n');
    push_section_title(&mut out, "STATUS LOG", ANSI_BLUE);
    out.push_str(&format!(
        "  {ANSI_DIM}Level{ANSI_RESET}: {status_color}{}{ANSI_RESET}\n",
        status.level
    ));
    for msg in status.messages {
        out.push_str(&format!("  {ANSI_DIM}•{ANSI_RESET} {msg}\n"));
    }
    out
}

fn push_model_section(out: &mut String, title: &str, rows: &[ModelStat], detailed: bool) {
    push_section_title(out, title, ANSI_GREEN);
    if detailed {
        out.push_str(&format!(
            "  {ANSI_DIM}MODEL      REQ    INPUT     OUTPUT    CACHE READ  ALL       COST{ANSI_RESET}\n"
        ));
        for row in sorted_models(rows).iter().take(8) {
            out.push_str(&format!(
                "  {ANSI_CYAN}{:<10}{ANSI_RESET} {:>5} {:>8} {:>8} {:>10} {:>8} {ANSI_GREEN}{:>10}{ANSI_RESET}\n",
                truncate(&row.model, 10),
                row.requests,
                compact_number(row.input_tokens),
                compact_number(row.output_tokens),
                compact_number(row.cache_read_tokens),
                compact_number(row.all_tokens),
                display_cost(row.cost, &row.formatted_cost)
            ));
        }
    } else {
        out.push_str(&format!(
            "  {ANSI_DIM}MODEL                       REQ        ALL        COST{ANSI_RESET}\n"
        ));
        for row in sorted_models(rows).iter().take(8) {
            out.push_str(&format!(
                "  {ANSI_CYAN}{:<27}{ANSI_RESET} {:>6} {:>10} {ANSI_GREEN}{:>10}{ANSI_RESET}\n",
                truncate(&row.model, 27),
                comma(row.requests),
                compact_number(row.all_tokens),
                display_cost(row.cost, &row.formatted_cost)
            ));
        }
    }
    if rows.is_empty() {
        out.push_str(&format!("  {ANSI_DIM}no usage records{ANSI_RESET}\n"));
    }
}

pub fn render_dashboard(
    frame: &mut Frame<'_>,
    snapshot: &Snapshot,
    refresh_secs: f64,
    masked_key: &str,
    detailed_daily: bool,
    detailed_monthly: bool,
    trend: &TrendHistory,
) {
    let status = build_status(snapshot);
    let color = match status.level.as_str() {
        "OK" => Color::Rgb(0, 255, 135),
        "WARN" => Color::Rgb(255, 216, 0),
        _ => Color::Rgb(255, 70, 70),
    };
    let root = frame.area();
    let trend_height = trend_height(root.height);
    let chunks = if let Some(trend_height) = trend_height {
        Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(3),
                Constraint::Length(8),
                Constraint::Length(trend_height),
                Constraint::Min(8),
                Constraint::Length(6),
            ])
            .split(root)
    } else {
        Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(3),
                Constraint::Length(8),
                Constraint::Min(8),
                Constraint::Length(8),
            ])
            .split(root)
    };
    let top = Paragraph::new(Line::from(vec![
        Span::styled(
            "▰ crstop ",
            Style::default()
                .fg(Color::Rgb(0, 215, 255))
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled(
            format!("{} ", status.level),
            Style::default().fg(color).add_modifier(Modifier::BOLD),
        ),
        Span::raw(format!(
            "CRS={} v{} Redis={} Key={} Refresh={}s Last={}",
            snapshot.health.status,
            snapshot.health.version.as_deref().unwrap_or("-"),
            snapshot.health.redis_status.as_deref().unwrap_or("-"),
            masked_key,
            format_secs(refresh_secs),
            snapshot.fetched_at
        )),
    ]))
    .style(Style::default().fg(Color::Rgb(220, 225, 235)))
    .block(panel_block(" neon status ", Color::Rgb(0, 215, 255)));
    frame.render_widget(top, chunks[0]);

    let mid = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
        .split(chunks[1]);
    frame.render_widget(total_widget(snapshot), mid[0]);
    frame.render_widget(limits_widget(snapshot), mid[1]);

    let table_area = if trend_height.is_some() {
        chunks[3]
    } else {
        chunks[2]
    };
    let log_area = if trend_height.is_some() {
        chunks[4]
    } else {
        chunks[3]
    };

    if trend_height.is_some() {
        render_trend_panel(frame, chunks[2], trend);
    }

    let tables = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
        .split(table_area);
    frame.render_widget(
        model_table("TODAY BY MODEL", &snapshot.daily, detailed_daily),
        tables[0],
    );
    frame.render_widget(
        model_table("MONTH BY MODEL", &snapshot.monthly, detailed_monthly),
        tables[1],
    );

    let log_text: Vec<Line> = status
        .messages
        .iter()
        .map(|m| {
            Line::from(vec![
                Span::styled("• ", Style::default().fg(Color::Rgb(0, 215, 255))),
                Span::raw(m.clone()),
            ])
        })
        .collect();
    frame.render_widget(
        Paragraph::new(log_text)
            .style(Style::default().fg(Color::Rgb(220, 225, 235)))
            .block(panel_block(" STATUS LOG ", Color::Rgb(60, 160, 255))),
        log_area,
    );
}

fn render_trend_panel(frame: &mut Frame<'_>, area: Rect, trend: &TrendHistory) {
    frame.render_widget(panel_block(" LIVE TREND ", Color::Rgb(0, 215, 255)), area);
    let inner = area.inner(Margin {
        horizontal: 1,
        vertical: 1,
    });
    let columns = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage(34),
            Constraint::Percentage(33),
            Constraint::Percentage(33),
        ])
        .split(inner);

    let bucket = Duration::from_secs(TREND_BUCKET_SECS);
    let request_buckets = trend.request_time_buckets(sparkline_width(columns[0]), bucket);
    let token_buckets = trend.token_time_buckets(sparkline_width(columns[1]), bucket);
    let cost_buckets = trend.cost_time_buckets(sparkline_width(columns[2]), bucket);

    render_trend_sparkline(
        frame,
        columns[0],
        request_buckets.clone(),
        format!(
            " REQ {} {} ",
            trend_window_label(&request_buckets, bucket),
            signed_count(Some(request_buckets.iter().copied().sum()))
        ),
        Color::Rgb(60, 160, 255),
    );
    render_trend_sparkline(
        frame,
        columns[1],
        token_buckets.clone(),
        format!(
            " TOKENS {} {} ",
            trend_window_label(&token_buckets, bucket),
            signed_compact(Some(token_buckets.iter().copied().sum()))
        ),
        Color::Rgb(0, 215, 255),
    );
    render_trend_sparkline(
        frame,
        columns[2],
        cost_buckets.clone(),
        format!(
            " COST {} {} ",
            trend_window_label(&cost_buckets, bucket),
            signed_cents(Some(cost_buckets.iter().copied().sum()))
        ),
        Color::Rgb(0, 255, 135),
    );
}

fn trend_height(total_height: u16) -> Option<u16> {
    if total_height < 30 {
        None
    } else {
        Some((total_height / 5).clamp(7, 18))
    }
}

fn sparkline_width(area: Rect) -> usize {
    area.width.saturating_sub(2).max(1) as usize
}

fn render_trend_sparkline(
    frame: &mut Frame<'_>,
    area: Rect,
    data: Vec<u64>,
    title: String,
    color: Color,
) {
    frame.render_widget(panel_block(&title, color), area);
    let inner = area.inner(Margin {
        horizontal: 1,
        vertical: 1,
    });
    if inner.is_empty() || data.is_empty() {
        return;
    }
    let max = data.iter().copied().max().unwrap_or(0).max(1);
    let max_columns = inner.width as usize;
    let visible_start = data.len().saturating_sub(max_columns);
    let visible = &data[visible_start..];
    let start_x = inner.right().saturating_sub(visible.len() as u16);
    let style = Style::default().fg(color);
    let buffer = frame.buffer_mut();

    for y in inner.top()..inner.bottom() {
        for x in inner.left()..inner.right() {
            if let Some(cell) = buffer.cell_mut((x, y)) {
                cell.set_symbol(" ");
            }
        }
    }

    for (i, value) in visible.iter().enumerate() {
        if *value == 0 {
            continue;
        }
        let height = ((*value as f64 / max as f64) * inner.height as f64)
            .ceil()
            .clamp(1.0, inner.height as f64) as u16;
        let x = start_x + i as u16;
        for y in inner.bottom().saturating_sub(height)..inner.bottom() {
            if let Some(cell) = buffer.cell_mut((x, y)) {
                cell.set_symbol(TREND_BAR_SYMBOL).set_style(style);
            }
        }
    }
}

fn total_widget(snapshot: &Snapshot) -> Paragraph<'_> {
    Paragraph::new(vec![
        metric_line(
            "Requests",
            comma(snapshot.user.total.requests),
            Color::Rgb(60, 160, 255),
        ),
        metric_line(
            "All Tokens",
            comma(snapshot.user.total.all_tokens),
            Color::Rgb(0, 215, 255),
        ),
        metric_line(
            "Input",
            comma(snapshot.user.total.input_tokens),
            Color::Rgb(180, 180, 255),
        ),
        metric_line(
            "Output",
            comma(snapshot.user.total.output_tokens),
            Color::Rgb(255, 140, 200),
        ),
        metric_line(
            "Cache Read",
            comma(snapshot.user.total.cache_read_tokens),
            Color::Rgb(0, 255, 180),
        ),
        metric_line(
            "Cost",
            display_cost(
                snapshot.user.total.cost,
                &snapshot.user.total.formatted_cost,
            ),
            Color::Rgb(0, 255, 135),
        ),
    ])
    .block(panel_block(" TOTAL ", Color::Rgb(0, 215, 255)))
}

fn limits_widget(snapshot: &Snapshot) -> Paragraph<'_> {
    Paragraph::new(vec![
        Line::from(vec![
            Span::styled("Daily Cost  ", label_style()),
            Span::styled(
                format!(
                    "{} / {}  {}",
                    dollars(snapshot.user.limits.current_daily_cost),
                    limit_cost(snapshot.user.limits.daily_cost_limit),
                    plain_usage_bar(
                        snapshot.user.limits.current_daily_cost,
                        snapshot.user.limits.daily_cost_limit,
                    )
                ),
                Style::default().fg(Color::Rgb(0, 255, 135)),
            ),
        ]),
        Line::from(vec![
            Span::styled("Total Cost  ", label_style()),
            Span::styled(
                format!(
                    "{} / {}  {}",
                    dollars(snapshot.user.limits.current_total_cost),
                    limit_cost(snapshot.user.limits.total_cost_limit),
                    plain_usage_bar(
                        snapshot.user.limits.current_total_cost,
                        snapshot.user.limits.total_cost_limit,
                    )
                ),
                Style::default().fg(Color::Rgb(0, 215, 255)),
            ),
        ]),
        metric_line(
            "Concurrency",
            limit_u64(snapshot.user.limits.concurrency_limit),
            Color::Rgb(255, 216, 0),
        ),
        metric_line(
            "Rate Limit",
            limit_u64(snapshot.user.limits.rate_limit_requests),
            Color::Rgb(255, 216, 0),
        ),
        metric_line(
            "Pool Limits",
            if snapshot.pool_visible {
                "visible".to_string()
            } else {
                "not visible without admin token".to_string()
            },
            Color::Rgb(255, 216, 0),
        ),
    ])
    .block(panel_block(" LIMITS ", Color::Rgb(190, 120, 255)))
}

fn model_table<'a>(title: &'a str, rows: &'a [ModelStat], detailed: bool) -> Table<'a> {
    let header = if detailed {
        Row::new(vec![
            "MODEL", "REQ", "INPUT", "OUTPUT", "CACHE", "ALL", "COST",
        ])
        .style(
            Style::default()
                .fg(Color::Rgb(0, 215, 255))
                .add_modifier(Modifier::BOLD),
        )
    } else {
        Row::new(vec!["MODEL", "REQ", "ALL", "COST"]).style(
            Style::default()
                .fg(Color::Rgb(0, 215, 255))
                .add_modifier(Modifier::BOLD),
        )
    };
    let table_rows: Vec<Row> = sorted_models(rows)
        .into_iter()
        .take(12)
        .map(|r| {
            if detailed {
                Row::new(vec![
                    Cell::from(r.model).style(Style::default().fg(Color::Rgb(0, 215, 255))),
                    Cell::from(comma(r.requests)),
                    Cell::from(compact_number(r.input_tokens)),
                    Cell::from(compact_number(r.output_tokens)),
                    Cell::from(compact_number(r.cache_read_tokens)),
                    Cell::from(compact_number(r.all_tokens)),
                    Cell::from(display_cost(r.cost, &r.formatted_cost))
                        .style(Style::default().fg(Color::Rgb(0, 255, 135))),
                ])
            } else {
                Row::new(vec![
                    Cell::from(r.model).style(Style::default().fg(Color::Rgb(0, 215, 255))),
                    Cell::from(comma(r.requests)),
                    Cell::from(compact_number(r.all_tokens)),
                    Cell::from(display_cost(r.cost, &r.formatted_cost))
                        .style(Style::default().fg(Color::Rgb(0, 255, 135))),
                ])
            }
        })
        .collect();
    let widths: Vec<Constraint> = if detailed {
        vec![
            Constraint::Min(12),
            Constraint::Length(7),
            Constraint::Length(8),
            Constraint::Length(8),
            Constraint::Length(8),
            Constraint::Length(8),
            Constraint::Length(10),
        ]
    } else {
        vec![
            Constraint::Min(16),
            Constraint::Length(7),
            Constraint::Length(10),
            Constraint::Length(10),
        ]
    };
    Table::new(table_rows, widths)
        .header(header)
        .row_highlight_style(Style::default().fg(Color::Rgb(255, 255, 255)))
        .style(Style::default().fg(Color::Rgb(220, 225, 235)))
        .block(panel_block(title, Color::Rgb(0, 255, 135)))
}

fn push_section_title(out: &mut String, title: &str, color: &str) {
    out.push_str(&format!("{color}╭─ {title} {ANSI_RESET}\n"));
}

fn ansi_status_color(level: &str) -> &'static str {
    match level {
        "OK" => ANSI_GREEN,
        "WARN" => ANSI_YELLOW,
        _ => ANSI_RED,
    }
}

fn usage_bar(current: f64, limit: f64) -> String {
    let plain = plain_usage_bar(current, limit);
    let color = if limit > 0.0 && current / limit >= 0.9 {
        ANSI_YELLOW
    } else {
        ANSI_CYAN
    };
    format!("{color}{plain}{ANSI_RESET}")
}

fn plain_usage_bar(current: f64, limit: f64) -> String {
    if limit <= 0.0 {
        return "▰▰▰▰▰▰▰▰ unlimited".to_string();
    }
    let ratio = (current / limit).clamp(0.0, 1.0);
    let filled = (ratio * 8.0).round() as usize;
    format!(
        "{}{} {:>3.0}%",
        "▰".repeat(filled),
        "▱".repeat(8 - filled),
        ratio * 100.0
    )
}

fn panel_block(title: &str, color: Color) -> Block<'_> {
    Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(color))
        .title(Span::styled(
            title.to_string(),
            Style::default().fg(color).add_modifier(Modifier::BOLD),
        ))
}

fn label_style() -> Style {
    Style::default().fg(Color::Rgb(145, 155, 170))
}

fn metric_line(label: &str, value: String, value_color: Color) -> Line<'static> {
    Line::from(vec![
        Span::styled(format!("{label:<12}"), label_style()),
        Span::styled(value, Style::default().fg(value_color)),
    ])
}

fn limit_cost(value: f64) -> String {
    if value <= 0.0 {
        "unlimited".to_string()
    } else {
        dollars(value)
    }
}

fn limit_u64(value: u64) -> String {
    if value == 0 {
        "unlimited".to_string()
    } else {
        value.to_string()
    }
}

fn truncate(value: &str, width: usize) -> String {
    if value.chars().count() <= width {
        value.to_string()
    } else {
        let mut s: String = value.chars().take(width.saturating_sub(1)).collect();
        s.push('…');
        s
    }
}

fn signed_count(value: Option<u64>) -> String {
    value
        .map(|v| format!("+{}", comma(v)))
        .unwrap_or_else(|| "collecting".to_string())
}

fn signed_compact(value: Option<u64>) -> String {
    value
        .map(|v| format!("+{}", compact_number(v)))
        .unwrap_or_else(|| "collecting".to_string())
}

fn signed_cents(value: Option<u64>) -> String {
    value
        .map(|cents| format!("+${:.2}", cents as f64 / 100.0))
        .unwrap_or_else(|| "collecting".to_string())
}

fn trend_window_label(_data: &[u64], bucket: Duration) -> String {
    let seconds = bucket.as_secs().max(1);
    if seconds >= 60 {
        format!("{}m/b", seconds / 60)
    } else {
        format!("{}s/b", seconds)
    }
}

fn format_secs(value: f64) -> String {
    if (value.fract()).abs() < f64::EPSILON {
        format!("{value:.0}")
    } else {
        format!("{value:.1}")
    }
}

#[allow(dead_code)]
fn _rect(width: u16, height: u16) -> Rect {
    Rect::new(0, 0, width, height)
}
