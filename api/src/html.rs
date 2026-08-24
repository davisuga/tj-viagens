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
    pub flight_info: String,
    pub price: String,
    pub issued_at: String,
}

pub struct ReportProposal {
    pub position: usize,
    pub supplier: String,
    pub cnpj: String,
    pub price: String,
    pub flight_info: String,
    pub submitted_at: String,
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
    pub status: String,
    pub origin: String,
    pub destination: String,
    pub passenger_name: String,
    pub passenger_cpf_masked: String,
    pub reference_price: String,
    pub notified: i64,
    pub proposals: Vec<ReportProposal>,
    pub has_economy: bool,
    pub economy_saved: String,
    pub economy_pct: String,
    pub os_number: String,
    pub ticket_line: String,
    pub audit_ok: bool,
    pub timeline: Vec<ReportEvent>,
    pub generated_at: String,
}
