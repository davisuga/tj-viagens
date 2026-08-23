use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum Role {
    Admin,
    Servidor,
    Fornecedor,
}

impl Role {
    pub fn as_str(&self) -> &'static str {
        match self {
            Role::Admin => "ADMIN",
            Role::Servidor => "SERVIDOR",
            Role::Fornecedor => "FORNECEDOR",
        }
    }
    pub fn parse(s: &str) -> Option<Role> {
        match s {
            "ADMIN" => Some(Role::Admin),
            "SERVIDOR" => Some(Role::Servidor),
            "FORNECEDOR" => Some(Role::Fornecedor),
            _ => None,
        }
    }
    pub fn is_staff(&self) -> bool {
        match self {
            Role::Admin | Role::Servidor => true,
            Role::Fornecedor => false,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum SupplierStatus {
    Pending,
    Active,
    Rejected,
    Suspended,
}

impl SupplierStatus {
    pub fn as_str(&self) -> &'static str {
        match self {
            SupplierStatus::Pending => "PENDING",
            SupplierStatus::Active => "ACTIVE",
            SupplierStatus::Rejected => "REJECTED",
            SupplierStatus::Suspended => "SUSPENDED",
        }
    }
    pub fn parse(s: &str) -> Option<SupplierStatus> {
        match s {
            "PENDING" => Some(SupplierStatus::Pending),
            "ACTIVE" => Some(SupplierStatus::Active),
            "REJECTED" => Some(SupplierStatus::Rejected),
            "SUSPENDED" => Some(SupplierStatus::Suspended),
            _ => None,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum QuotationStatus {
    Draft,
    Open,
    Closed,
    Awarded,
    Ticketed,
    Completed,
}

impl QuotationStatus {
    pub fn as_str(&self) -> &'static str {
        match self {
            QuotationStatus::Draft => "DRAFT",
            QuotationStatus::Open => "OPEN",
            QuotationStatus::Closed => "CLOSED",
            QuotationStatus::Awarded => "AWARDED",
            QuotationStatus::Ticketed => "TICKETED",
            QuotationStatus::Completed => "COMPLETED",
        }
    }
    pub fn parse(s: &str) -> Option<QuotationStatus> {
        match s {
            "DRAFT" => Some(QuotationStatus::Draft),
            "OPEN" => Some(QuotationStatus::Open),
            "CLOSED" => Some(QuotationStatus::Closed),
            "AWARDED" => Some(QuotationStatus::Awarded),
            "TICKETED" => Some(QuotationStatus::Ticketed),
            "COMPLETED" => Some(QuotationStatus::Completed),
            _ => None,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum DocType {
    ContratoSocial,
    CndFederal,
    CrfFgts,
    Cndt,
}

impl DocType {
    pub fn as_str(&self) -> &'static str {
        match self {
            DocType::ContratoSocial => "CONTRATO_SOCIAL",
            DocType::CndFederal => "CND_FEDERAL",
            DocType::CrfFgts => "CRF_FGTS",
            DocType::Cndt => "CNDT",
        }
    }
    pub fn parse(s: &str) -> Option<DocType> {
        match s {
            "CONTRATO_SOCIAL" => Some(DocType::ContratoSocial),
            "CND_FEDERAL" => Some(DocType::CndFederal),
            "CRF_FGTS" => Some(DocType::CrfFgts),
            "CNDT" => Some(DocType::Cndt),
            _ => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn enum_string_round_trips_are_case_sensitive() {
        for role in [Role::Admin, Role::Servidor, Role::Fornecedor] {
            assert_eq!(Role::parse(role.as_str()), Some(role));
        }
        for status in [
            SupplierStatus::Pending,
            SupplierStatus::Active,
            SupplierStatus::Rejected,
            SupplierStatus::Suspended,
        ] {
            assert_eq!(SupplierStatus::parse(status.as_str()), Some(status));
        }
        for status in [
            QuotationStatus::Draft,
            QuotationStatus::Open,
            QuotationStatus::Closed,
            QuotationStatus::Awarded,
            QuotationStatus::Ticketed,
            QuotationStatus::Completed,
        ] {
            assert_eq!(QuotationStatus::parse(status.as_str()), Some(status));
        }
        for doc in [DocType::ContratoSocial, DocType::CndFederal, DocType::CrfFgts, DocType::Cndt] {
            assert_eq!(DocType::parse(doc.as_str()), Some(doc));
        }
        assert_eq!(Role::parse("admin"), None);
        assert_eq!(DocType::parse("cndt"), None);
    }
}
