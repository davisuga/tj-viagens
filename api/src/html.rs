use askama::Template;

#[derive(Template)]
#[template(path = "os.html")]
pub struct OsTemplate {
    pub number: String,
    pub code: String,
    pub supplier_name: String,
    pub supplier_cnpj: String,
    pub passenger_name: String,
    pub passenger_cpf: String,
    pub passenger_sex: String,
    pub passenger_birth: String,
    pub origin: String,
    pub destination: String,
    pub departure_at: String,
    pub itinerary: String,
    pub price: String,
    pub issued_at: String,
    pub sei_process: String,
}

pub struct ReportProposal {
    pub position: usize,
    pub supplier: String,
    pub cnpj: String,
    pub price: String,
    pub itinerary: String,
    /// Eligibility as frozen at the award; empty before the award.
    pub habilitacao: String,
    pub habilitacao_class: &'static str,
    pub submitted_at: String,
}

pub struct ReportDoc {
    pub label: String,
    pub validity: String,
}

pub struct ReportEvent {
    pub seq: i64,
    pub at: String,
    pub event_type: String,
}

#[derive(Template)]
#[template(path = "report.html")]
pub struct ReportTemplate {
    pub code: String,
    pub sei_process: String,
    pub status: String,
    pub origin: String,
    pub destination: String,
    pub passenger_name: String,
    pub passenger_cpf_masked: String,
    pub reference_price: String,
    pub notified: i64,
    pub proposals: Vec<ReportProposal>,
    pub has_decision: bool,
    pub decision_winner: String,
    pub decision_reason: String,
    /// Empty when the winner is the cheapest eligible bid.
    pub decision_comparison: String,
    pub decision_justification: String,
    /// Cheaper bids set aside because the supplier was not eligible.
    pub discarded: Vec<String>,
    pub winner_docs: Vec<ReportDoc>,
    pub has_economy: bool,
    pub economy_saved: String,
    pub economy_pct: String,
    pub os_number: String,
    pub ticket_line: String,
    pub audit_ok: bool,
    pub timeline: Vec<ReportEvent>,
    pub generated_at: String,
}
