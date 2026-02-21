use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph, Clear},
    Frame,
};

use crate::app::*;

pub fn draw(f: &mut Frame, app: &App) {
    match app.phase {
        AppPhase::Setup => draw_setup(f, app),
        AppPhase::Running => draw_running(f, app),
        AppPhase::Celebration => draw_celebration(f, app),
    }
}

fn draw_setup(f: &mut Frame, app: &App) {
    let area = f.area();
    f.render_widget(Clear, area);

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3),
            Constraint::Min(10),
            Constraint::Length(3),
        ])
        .split(area);

    // Title
    let title = Paragraph::new(Line::from(vec![
        Span::styled("  profit-cli ", Style::default().fg(Color::Green).add_modifier(Modifier::BOLD)),
        Span::raw("— Setup"),
    ]))
    .block(Block::default().borders(Borders::ALL).border_style(Style::default().fg(Color::Green)));
    f.render_widget(title, chunks[0]);

    match app.setup_step {
        SetupStep::Currency => draw_currency_select(f, app, chunks[1]),
        SetupStep::ProviderSelect => draw_provider_select(f, app, chunks[1]),
        SetupStep::ProviderApiKey => draw_api_key_input(f, app, chunks[1]),
        SetupStep::ProviderMerchantAccount => draw_merchant_input(f, app, chunks[1]),
        SetupStep::Confirm => draw_confirm(f, app, chunks[1]),
    }

    // Help
    let help_text = match app.setup_step {
        SetupStep::Currency => "↑↓ select  Enter confirm  q quit",
        SetupStep::ProviderSelect => "↑↓ select  Space toggle  Enter continue  q quit",
        SetupStep::ProviderApiKey | SetupStep::ProviderMerchantAccount => "Type API key  Enter confirm  Esc back",
        SetupStep::Confirm => "Enter start  Esc back",
    };
    let help = Paragraph::new(help_text)
        .style(Style::default().fg(Color::DarkGray))
        .block(Block::default().borders(Borders::ALL));
    f.render_widget(help, chunks[2]);
}

fn draw_currency_select(f: &mut Frame, app: &App, area: Rect) {
    let mut lines = vec![
        Line::from(Span::styled("Select your currency:", Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD))),
        Line::from(""),
    ];

    for (i, (code, symbol)) in CURRENCIES.iter().enumerate() {
        let marker = if i == app.setup_currency_idx { "▸ " } else { "  " };
        let style = if i == app.setup_currency_idx {
            Style::default().fg(Color::Green).add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(Color::White)
        };
        lines.push(Line::from(Span::styled(
            format!("{}{} ({})", marker, code, symbol),
            style,
        )));
    }

    let p = Paragraph::new(lines).block(Block::default().borders(Borders::ALL));
    f.render_widget(p, area);
}

fn draw_provider_select(f: &mut Frame, app: &App, area: Rect) {
    let mut lines = vec![
        Line::from(Span::styled("Select payment providers:", Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD))),
        Line::from(""),
    ];

    for (i, prov) in app.provider_configs.iter().enumerate() {
        let marker = if i == app.setup_cursor { "▸ " } else { "  " };
        let check = if prov.enabled { "[✓]" } else { "[ ]" };
        let style = if i == app.setup_cursor {
            Style::default().fg(Color::Green).add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(Color::White)
        };
        lines.push(Line::from(Span::styled(
            format!("{}{} {}", marker, check, prov.name),
            style,
        )));
    }

    let p = Paragraph::new(lines).block(Block::default().borders(Borders::ALL));
    f.render_widget(p, area);
}

fn draw_api_key_input(f: &mut Frame, app: &App, area: Rect) {
    let prov = &app.provider_configs[app.current_provider_idx];
    let masked: String = if app.setup_input.is_empty() {
        String::new()
    } else {
        let len = app.setup_input.len();
        if len <= 4 {
            "*".repeat(len)
        } else {
            format!("{}{}",  "*".repeat(len - 4), &app.setup_input[len-4..])
        }
    };

    let lines = vec![
        Line::from(Span::styled(
            format!("Enter API key for {}:", prov.name),
            Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD),
        )),
        Line::from(""),
        Line::from(Span::styled(
            format!("▸ {}_", masked),
            Style::default().fg(Color::Green),
        )),
    ];

    let p = Paragraph::new(lines).block(Block::default().borders(Borders::ALL));
    f.render_widget(p, area);
}

fn draw_merchant_input(f: &mut Frame, app: &App, area: Rect) {
    let prov = &app.provider_configs[app.current_provider_idx];
    let lines = vec![
        Line::from(Span::styled(
            format!("Enter Merchant Account for {}:", prov.name),
            Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD),
        )),
        Line::from(""),
        Line::from(Span::styled(
            format!("▸ {}_", app.setup_input),
            Style::default().fg(Color::Green),
        )),
    ];

    let p = Paragraph::new(lines).block(Block::default().borders(Borders::ALL));
    f.render_widget(p, area);
}

fn draw_confirm(f: &mut Frame, app: &App, area: Rect) {
    let enabled: Vec<&ProviderSetupState> = app.provider_configs.iter().filter(|p| p.enabled).collect();
    let mut lines = vec![
        Line::from(Span::styled("Ready to go!", Style::default().fg(Color::Green).add_modifier(Modifier::BOLD))),
        Line::from(""),
        Line::from(format!("Currency: {} ({})", app.config.currency, app.config.currency_symbol)),
        Line::from(format!("Providers: {}", enabled.iter().map(|p| p.name.as_str()).collect::<Vec<_>>().join(", "))),
        Line::from(""),
        Line::from(Span::styled("Press Enter to start watching payments!", Style::default().fg(Color::Yellow))),
    ];

    if let Some(ref err) = app.error_message {
        lines.push(Line::from(""));
        lines.push(Line::from(Span::styled(err.clone(), Style::default().fg(Color::Red))));
    }

    let p = Paragraph::new(lines).block(Block::default().borders(Borders::ALL));
    f.render_widget(p, area);
}

fn draw_running(f: &mut Frame, app: &App) {
    let area = f.area();
    f.render_widget(Clear, area);

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3),
            Constraint::Min(5),
            Constraint::Length(3),
        ])
        .split(area);

    // Header with total
    let dur = app.session_duration();
    let minutes = dur.num_minutes();
    let seconds = dur.num_seconds() % 60;
    let total_display = format_money(app.total_cents, &app.config.currency_symbol);

    let header = Paragraph::new(Line::from(vec![
        Span::styled("  profit-cli ", Style::default().fg(Color::Green).add_modifier(Modifier::BOLD)),
        Span::raw("│ "),
        Span::styled(total_display, Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)),
        Span::raw(format!(" │ {}m {}s", minutes, seconds)),
        Span::raw(format!(" │ {} payments", app.session_payments.len())),
    ]))
    .block(Block::default().borders(Borders::ALL).border_style(Style::default().fg(Color::Green)));
    f.render_widget(header, chunks[0]);

    // Bill stacking area
    draw_bills(f, app, chunks[1]);

    // Status bar
    let providers: Vec<String> = app.config.providers.iter().map(|p| p.provider.clone()).collect();
    let pending = app.pending_bills.len();
    let status_text = if pending > 0 {
        format!(" {} │ +{} incoming", providers.join(" + "), pending)
    } else {
        format!(" {} │ Watching for payments...", providers.join(" + "))
    };
    let status = Paragraph::new(status_text)
        .style(Style::default().fg(Color::DarkGray))
        .block(Block::default().borders(Borders::ALL));
    f.render_widget(status, chunks[2]);
}

fn draw_bills(f: &mut Frame, app: &App, area: Rect) {
    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::DarkGray));
    let inner = block.inner(area);
    f.render_widget(block, area);

    if app.bills.is_empty() && app.pending_bills.is_empty() {
        let waiting = Paragraph::new(Line::from(vec![
            Span::styled("  Waiting for payments", Style::default().fg(Color::DarkGray)),
            Span::styled(
                dots_animation(app.celebration_tick),
                Style::default().fg(Color::DarkGray),
            ),
        ]));
        let centered = centered_rect(40, 3, inner);
        f.render_widget(waiting, centered);
        return;
    }

    let sym = &app.config.currency_symbol;
    for bill in &app.bills {
        let y = bill.y_pos as u16;
        if y >= inner.height || y < inner.y {
            continue;
        }

        let bill_y = inner.y + y;
        if bill_y + 2 >= inner.y + inner.height {
            continue;
        }

        let glow = if !bill.settled { Color::Yellow } else if bill.age_ticks < 10 { Color::Green } else { Color::DarkGray };
        let bill_style = Style::default().fg(glow);

        let bill_width = 22u16.min(inner.width);
        let x = inner.x + (inner.width.saturating_sub(bill_width)) / 2;

        let bill_area = Rect::new(x, bill_y, bill_width, 2);

        let top = format!("┌{}┐", "─".repeat((bill_width - 2) as usize));
        let mid = format!("│{} {}1{} │",
            " ".repeat(1),
            sym,
            " ".repeat((bill_width as usize).saturating_sub(sym.len() + 6)),
        );

        let bill_text = Paragraph::new(vec![
            Line::from(Span::styled(&top, bill_style)),
            Line::from(Span::styled(&mid, bill_style)),
        ]);

        if bill_area.y + bill_area.height <= inner.y + inner.height {
            f.render_widget(bill_text, bill_area);
        }
    }
}

fn draw_celebration(f: &mut Frame, app: &App) {
    let area = f.area();
    f.render_widget(Clear, area);

    let tick = app.celebration_tick;
    let sparkle = if tick % 4 < 2 { "✨" } else { "🎉" };
    let border_color = match tick % 6 {
        0 => Color::Green,
        1 => Color::Yellow,
        2 => Color::Cyan,
        3 => Color::Magenta,
        4 => Color::Red,
        _ => Color::Blue,
    };

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Percentage(20),
            Constraint::Percentage(60),
            Constraint::Percentage(20),
        ])
        .split(area);

    let dur = app.session_duration();
    let total = format_money(app.total_cents, &app.config.currency_symbol);
    let avg = if !app.session_payments.is_empty() {
        format_money(app.total_cents / app.session_payments.len() as i64, &app.config.currency_symbol)
    } else {
        format_money(0, &app.config.currency_symbol)
    };

    let celebration_art = vec![
        Line::from(""),
        Line::from(Span::styled(
            format!("  {} SCREEN FULL! {} ", sparkle, sparkle),
            Style::default().fg(border_color).add_modifier(Modifier::BOLD),
        )),
        Line::from(""),
        Line::from(Span::styled(
            "  ╔══════════════════════════════╗",
            Style::default().fg(Color::Yellow),
        )),
        Line::from(Span::styled(
            format!("  ║   Total: {:>17}  ║", total),
            Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD),
        )),
        Line::from(Span::styled(
            "  ╠══════════════════════════════╣",
            Style::default().fg(Color::Yellow),
        )),
        Line::from(Span::styled(
            format!("  ║   Payments: {:>14}  ║", app.session_payments.len()),
            Style::default().fg(Color::Green),
        )),
        Line::from(Span::styled(
            format!("  ║   Average: {:>15}  ║", avg),
            Style::default().fg(Color::Green),
        )),
        Line::from(Span::styled(
            format!("  ║   Duration: {:>11}m {:>2}s  ║", dur.num_minutes(), dur.num_seconds() % 60),
            Style::default().fg(Color::Green),
        )),
        Line::from(Span::styled(
            format!("  ║   Rate: {:>13}/min  ║",
                if dur.num_minutes() > 0 {
                    format_money(app.total_cents / dur.num_minutes(), &app.config.currency_symbol)
                } else {
                    total.clone()
                }),
            Style::default().fg(Color::Green),
        )),
        Line::from(Span::styled(
            "  ╚══════════════════════════════╝",
            Style::default().fg(Color::Yellow),
        )),
        Line::from(""),
        Line::from(Span::styled(
            "  Resetting in a moment...",
            Style::default().fg(Color::DarkGray),
        )),
    ];

    let p = Paragraph::new(celebration_art)
        .block(Block::default().borders(Borders::ALL).border_style(Style::default().fg(border_color)));
    f.render_widget(p, chunks[1]);
}

fn format_money(cents: i64, symbol: &str) -> String {
    let whole = cents / 100;
    let frac = (cents % 100).abs();
    // Add thousand separators
    let whole_str = {
        let s = whole.to_string();
        let mut result = String::new();
        for (i, c) in s.chars().rev().enumerate() {
            if i > 0 && i % 3 == 0 && c != '-' {
                result.push(',');
            }
            result.push(c);
        }
        result.chars().rev().collect::<String>()
    };
    format!("{}{}.{:02}", symbol, whole_str, frac)
}

fn dots_animation(tick: u32) -> String {
    match tick % 4 {
        0 => "".to_string(),
        1 => ".".to_string(),
        2 => "..".to_string(),
        _ => "...".to_string(),
    }
}

fn centered_rect(width: u16, height: u16, area: Rect) -> Rect {
    let x = area.x + (area.width.saturating_sub(width)) / 2;
    let y = area.y + (area.height.saturating_sub(height)) / 2;
    Rect::new(x, y, width.min(area.width), height.min(area.height))
}
