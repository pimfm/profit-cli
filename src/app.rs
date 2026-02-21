use crate::config::AppConfig;
use crate::psp::Payment;

#[derive(Debug, Clone)]
#[allow(dead_code)]
pub struct BillAnimation {
    pub amount_cents: i64,
    pub y_pos: f64,
    pub target_y: f64,
    pub settled: bool,
    pub age_ticks: u32,
    pub provider: String,
}

#[derive(Debug, Clone, PartialEq)]
pub enum AppPhase {
    Setup,
    Running,
    Celebration,
}

pub struct App {
    pub config: AppConfig,
    pub phase: AppPhase,
    pub bills: Vec<BillAnimation>,
    pub total_cents: i64,
    pub session_payments: Vec<Payment>,
    pub start_time: chrono::DateTime<chrono::Utc>,
    pub seen_ids: std::collections::HashSet<String>,
    pub celebration_tick: u32,
    pub setup_cursor: usize,
    pub setup_currency_idx: usize,
    pub setup_input: String,
    pub setup_step: SetupStep,
    pub provider_configs: Vec<ProviderSetupState>,
    pub current_provider_idx: usize,
    pub error_message: Option<String>,
    pub pending_bills: Vec<PendingBill>,
}

#[derive(Debug, Clone)]
pub struct PendingBill {
    pub amount_cents: i64,
    pub provider: String,
}

#[derive(Debug, Clone, PartialEq)]
pub enum SetupStep {
    Currency,
    ProviderSelect,
    ProviderApiKey,
    ProviderMerchantAccount,
    Confirm,
}

#[derive(Debug, Clone)]
pub struct ProviderSetupState {
    pub name: String,
    pub enabled: bool,
    pub api_key: String,
    pub merchant_account: String,
}

pub const CURRENCIES: &[(&str, &str)] = &[
    ("EUR", "€"),
    ("USD", "$"),
    ("GBP", "£"),
    ("JPY", "¥"),
    ("CHF", "CHF"),
    ("CAD", "CA$"),
    ("AUD", "A$"),
];

pub const PROVIDERS: &[&str] = &["Mock", "Mollie", "Adyen"];

impl App {
    pub fn new() -> Self {
        Self {
            config: AppConfig::default(),
            phase: AppPhase::Setup,
            bills: Vec::new(),
            total_cents: 0,
            session_payments: Vec::new(),
            start_time: chrono::Utc::now(),
            seen_ids: std::collections::HashSet::new(),
            celebration_tick: 0,
            setup_cursor: 0,
            setup_currency_idx: 0,
            setup_input: String::new(),
            setup_step: SetupStep::Currency,
            provider_configs: PROVIDERS.iter().map(|name| ProviderSetupState {
                name: name.to_string(),
                enabled: false,
                api_key: String::new(),
                merchant_account: String::new(),
            }).collect(),
            current_provider_idx: 0,
            error_message: None,
            pending_bills: Vec::new(),
        }
    }

    pub fn from_config(config: AppConfig) -> Self {
        let mut app = Self::new();
        app.config = config.clone();
        // Skip setup if already configured with at least one provider
        if !config.providers.is_empty() {
            app.phase = AppPhase::Running;
            app.start_time = chrono::Utc::now();
        }
        app
    }

    pub fn add_payment(&mut self, payment: Payment) {
        if self.seen_ids.contains(&payment.id) {
            return;
        }
        self.seen_ids.insert(payment.id.clone());
        self.total_cents += payment.amount_cents;

        // Queue bills: one bill per currency unit
        let units = (payment.amount_cents as f64 / 100.0).floor() as i64;
        for _ in 0..units.min(10) {
            self.pending_bills.push(PendingBill {
                amount_cents: 100,
                provider: payment.provider.clone(),
            });
        }

        self.session_payments.push(payment);
    }

    pub fn spawn_next_bill(&mut self, terminal_height: u16) {
        if self.pending_bills.is_empty() {
            return;
        }

        let pb = self.pending_bills.remove(0);
        let stack_y = self.calculate_stack_position(terminal_height);

        self.bills.push(BillAnimation {
            amount_cents: pb.amount_cents,
            y_pos: 0.0,
            target_y: stack_y as f64,
            settled: false,
            age_ticks: 0,
            provider: pb.provider,
        });
    }

    fn calculate_stack_position(&self, terminal_height: u16) -> u16 {
        let bill_height = 3u16;
        let floor = terminal_height.saturating_sub(4);
        let settled_count = self.bills.iter().filter(|b| b.settled).count() as u16;
        floor.saturating_sub(settled_count * bill_height)
    }

    pub fn tick_animations(&mut self) {
        for bill in &mut self.bills {
            if !bill.settled {
                let distance = bill.target_y - bill.y_pos;
                if distance.abs() < 1.0 {
                    bill.y_pos = bill.target_y;
                    bill.settled = true;
                } else {
                    bill.y_pos += distance * 0.3;
                }
            }
            bill.age_ticks += 1;
        }
    }

    pub fn is_screen_full(&self, terminal_height: u16) -> bool {
        let bill_height = 3u16;
        let settled = self.bills.iter().filter(|b| b.settled).count() as u16;
        settled * bill_height >= terminal_height.saturating_sub(6)
    }

    pub fn reset_session(&mut self) {
        self.bills.clear();
        self.pending_bills.clear();
        self.celebration_tick = 0;
        self.phase = AppPhase::Running;
        // Keep total and seen_ids so we don't recount
    }

    pub fn session_duration(&self) -> chrono::Duration {
        chrono::Utc::now() - self.start_time
    }
}
