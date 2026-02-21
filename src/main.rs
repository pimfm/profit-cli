mod app;
mod config;
mod psp;
mod ui;

use anyhow::Result;
use app::*;
use config::*;
use crossterm::{
    event::{self, Event, KeyCode, KeyEventKind},
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
    execute,
};
use psp::{PaymentProvider, PspConfig};
use ratatui::prelude::*;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::mpsc;

fn build_providers(configs: &[PspConfig]) -> Vec<Arc<dyn PaymentProvider>> {
    let mut providers: Vec<Arc<dyn PaymentProvider>> = Vec::new();
    for cfg in configs {
        match cfg.provider.as_str() {
            "Mock" => {
                providers.push(Arc::new(psp::mock::MockProvider::new()));
            }
            "Adyen" => {
                // Adyen needs merchant account — stored as "key|merchant"
                let parts: Vec<&str> = cfg.api_key.splitn(2, '|').collect();
                if parts.len() == 2 {
                    providers.push(Arc::new(psp::adyen::AdyenProvider::new(
                        parts[0].to_string(),
                        parts[1].to_string(),
                    )));
                }
            }
            _ => {}
        }
    }
    providers
}

#[tokio::main]
async fn main() -> Result<()> {
    enable_raw_mode()?;
    let mut stdout = std::io::stdout();
    execute!(stdout, EnterAlternateScreen)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    let result = run_app(&mut terminal).await;

    disable_raw_mode()?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen)?;
    terminal.show_cursor()?;

    if let Err(e) = result {
        eprintln!("Error: {}", e);
    }

    Ok(())
}

async fn run_app(terminal: &mut Terminal<CrosstermBackend<std::io::Stdout>>) -> Result<()> {
    let mut app = match load_config() {
        Some(cfg) => App::from_config(cfg),
        None => App::new(),
    };

    let (tx, mut rx) = mpsc::unbounded_channel::<Vec<psp::Payment>>();
    let mut poll_handle: Option<tokio::task::JoinHandle<()>> = None;
    let mut tick_count: u32 = 0;

    loop {
        terminal.draw(|f| ui::draw(f, &app))?;

        // Handle incoming payments from background task
        while let Ok(payments) = rx.try_recv() {
            for p in payments {
                app.add_payment(p);
            }
        }

        // Spawn pending bills with stagger
        if tick_count % 3 == 0 && !app.pending_bills.is_empty() && app.phase == AppPhase::Running {
            let h = terminal.size()?.height;
            app.spawn_next_bill(h);
        }

        // Tick animations
        if app.phase == AppPhase::Running {
            app.tick_animations();

            // Check if screen is full
            let h = terminal.size()?.height;
            if app.is_screen_full(h) && app.pending_bills.is_empty() {
                app.phase = AppPhase::Celebration;
                app.celebration_tick = 0;
            }
        }

        // Celebration timer
        if app.phase == AppPhase::Celebration {
            app.celebration_tick += 1;
            if app.celebration_tick > 100 {
                app.reset_session();
            }
        }

        // Waiting animation tick
        if app.phase == AppPhase::Running && app.bills.is_empty() {
            app.celebration_tick = tick_count; // reuse for dots animation
        }

        tick_count = tick_count.wrapping_add(1);

        // Poll events with short timeout for smooth animation
        if event::poll(Duration::from_millis(50))? {
            if let Event::Key(key) = event::read()? {
                if key.kind != KeyEventKind::Press {
                    continue;
                }

                match app.phase {
                    AppPhase::Setup => {
                        if handle_setup_input(&mut app, key.code) {
                            // Setup complete — save config and start polling
                            save_config(&app.config)?;
                            app.phase = AppPhase::Running;
                            app.start_time = chrono::Utc::now();

                            let providers = build_providers(&app.config.providers);
                            let tx2 = tx.clone();
                            let since = app.start_time;
                            poll_handle = Some(tokio::spawn(async move {
                                poll_payments(providers, tx2, since).await;
                            }));
                        }
                        if matches!(key.code, KeyCode::Char('q')) && matches!(app.setup_step, SetupStep::Currency | SetupStep::ProviderSelect) {
                            break;
                        }
                    }
                    AppPhase::Running => {
                        if matches!(key.code, KeyCode::Char('q') | KeyCode::Esc) {
                            break;
                        }
                    }
                    AppPhase::Celebration => {
                        if matches!(key.code, KeyCode::Char('q') | KeyCode::Esc) {
                            break;
                        }
                        if matches!(key.code, KeyCode::Enter | KeyCode::Char(' ')) {
                            app.reset_session();
                        }
                    }
                }
            }
        }

        // Start polling if we transitioned to Running from a loaded config
        if app.phase == AppPhase::Running && poll_handle.is_none() && !app.config.providers.is_empty() {
            let providers = build_providers(&app.config.providers);
            let tx2 = tx.clone();
            let since = app.start_time;
            poll_handle = Some(tokio::spawn(async move {
                poll_payments(providers, tx2, since).await;
            }));
        }
    }

    if let Some(h) = poll_handle {
        h.abort();
    }

    Ok(())
}

fn handle_setup_input(app: &mut App, key: KeyCode) -> bool {
    match app.setup_step {
        SetupStep::Currency => {
            match key {
                KeyCode::Up => {
                    if app.setup_currency_idx > 0 {
                        app.setup_currency_idx -= 1;
                    }
                }
                KeyCode::Down => {
                    if app.setup_currency_idx < CURRENCIES.len() - 1 {
                        app.setup_currency_idx += 1;
                    }
                }
                KeyCode::Enter => {
                    let (code, sym) = CURRENCIES[app.setup_currency_idx];
                    app.config.currency = code.to_string();
                    app.config.currency_symbol = sym.to_string();
                    app.setup_step = SetupStep::ProviderSelect;
                    app.setup_cursor = 0;
                }
                _ => {}
            }
        }
        SetupStep::ProviderSelect => {
            match key {
                KeyCode::Up => {
                    if app.setup_cursor > 0 {
                        app.setup_cursor -= 1;
                    }
                }
                KeyCode::Down => {
                    if app.setup_cursor < app.provider_configs.len() - 1 {
                        app.setup_cursor += 1;
                    }
                }
                KeyCode::Char(' ') => {
                    app.provider_configs[app.setup_cursor].enabled =
                        !app.provider_configs[app.setup_cursor].enabled;
                }
                KeyCode::Enter => {
                    let any_enabled = app.provider_configs.iter().any(|p| p.enabled);
                    if any_enabled {
                        // Find first enabled provider that needs API key (Mock doesn't)
                        if let Some(idx) = app.provider_configs.iter().position(|p| p.enabled && p.name != "Mock" && p.api_key.is_empty()) {
                            app.current_provider_idx = idx;
                            app.setup_input.clear();
                            app.setup_step = SetupStep::ProviderApiKey;
                        } else {
                            app.setup_step = SetupStep::Confirm;
                        }
                    }
                }
                _ => {}
            }
        }
        SetupStep::ProviderApiKey => {
            match key {
                KeyCode::Char(c) => {
                    app.setup_input.push(c);
                }
                KeyCode::Backspace => {
                    app.setup_input.pop();
                }
                KeyCode::Enter => {
                    if !app.setup_input.is_empty() {
                        app.provider_configs[app.current_provider_idx].api_key = app.setup_input.clone();
                        app.setup_input.clear();

                        // Adyen needs merchant account
                        if app.provider_configs[app.current_provider_idx].name == "Adyen" {
                            app.setup_step = SetupStep::ProviderMerchantAccount;
                        } else {
                            // Check for more providers needing keys
                            advance_to_next_provider_or_confirm(app);
                        }
                    }
                }
                KeyCode::Esc => {
                    app.setup_input.clear();
                    app.setup_step = SetupStep::ProviderSelect;
                }
                _ => {}
            }
        }
        SetupStep::ProviderMerchantAccount => {
            match key {
                KeyCode::Char(c) => {
                    app.setup_input.push(c);
                }
                KeyCode::Backspace => {
                    app.setup_input.pop();
                }
                KeyCode::Enter => {
                    if !app.setup_input.is_empty() {
                        app.provider_configs[app.current_provider_idx].merchant_account = app.setup_input.clone();
                        app.setup_input.clear();
                        advance_to_next_provider_or_confirm(app);
                    }
                }
                KeyCode::Esc => {
                    app.setup_input.clear();
                    app.setup_step = SetupStep::ProviderApiKey;
                }
                _ => {}
            }
        }
        SetupStep::Confirm => {
            match key {
                KeyCode::Enter => {
                    // Build final config
                    app.config.providers.clear();
                    for prov in &app.provider_configs {
                        if prov.enabled {
                            let api_key = if prov.name == "Adyen" {
                                format!("{}|{}", prov.api_key, prov.merchant_account)
                            } else {
                                prov.api_key.clone()
                            };
                            app.config.providers.push(PspConfig {
                                provider: prov.name.clone(),
                                api_key,
                            });
                        }
                    }
                    return true; // Setup complete
                }
                KeyCode::Esc => {
                    app.setup_step = SetupStep::ProviderSelect;
                }
                _ => {}
            }
        }
    }
    false
}

fn advance_to_next_provider_or_confirm(app: &mut App) {
    let start = app.current_provider_idx + 1;
    if let Some(idx) = app.provider_configs[start..].iter().position(|p| p.enabled && p.name != "Mock" && p.api_key.is_empty()) {
        app.current_provider_idx = start + idx;
        app.setup_input.clear();
        app.setup_step = SetupStep::ProviderApiKey;
    } else {
        app.setup_step = SetupStep::Confirm;
    }
}

async fn poll_payments(
    providers: Vec<Arc<dyn PaymentProvider>>,
    tx: mpsc::UnboundedSender<Vec<psp::Payment>>,
    since: chrono::DateTime<chrono::Utc>,
) {
    let mut interval = tokio::time::interval(Duration::from_secs(10));
    loop {
        interval.tick().await;

        for provider in &providers {
            match provider.fetch_recent_payments(since).await {
                Ok(payments) if !payments.is_empty() => {
                    if tx.send(payments).is_err() {
                        return;
                    }
                }
                Err(e) => {
                    eprintln!("Poll error from {}: {}", provider.name(), e);
                }
                _ => {}
            }
        }
    }
}
