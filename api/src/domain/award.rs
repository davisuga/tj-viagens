//! Award decision rule.
//!
//! The TJ does not always buy the cheapest seat: from Roraima the cheapest flight
//! to Brasília often adds a connection and hours of travel. Picking another bid is
//! legitimate, but it has to say why, and the stated reason has to hold against
//! the bids — otherwise the dossiê attached to SEI records something false.

use uuid::Uuid;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AwardReason {
    MenorPreco,
    MenorDuracao,
    MenosConexoes,
    HorarioCompativel,
    Outro,
}

impl AwardReason {
    pub fn as_str(&self) -> &'static str {
        match self {
            AwardReason::MenorPreco => "MENOR_PRECO",
            AwardReason::MenorDuracao => "MENOR_DURACAO",
            AwardReason::MenosConexoes => "MENOS_CONEXOES",
            AwardReason::HorarioCompativel => "HORARIO_COMPATIVEL",
            AwardReason::Outro => "OUTRO",
        }
    }

    pub fn parse(s: &str) -> Option<AwardReason> {
        match s {
            "MENOR_PRECO" => Some(AwardReason::MenorPreco),
            "MENOR_DURACAO" => Some(AwardReason::MenorDuracao),
            "MENOS_CONEXOES" => Some(AwardReason::MenosConexoes),
            "HORARIO_COMPATIVEL" => Some(AwardReason::HorarioCompativel),
            "OUTRO" => Some(AwardReason::Outro),
            _ => None,
        }
    }

    /// Wording printed in the dossiê. Every comparison is against the cheapest
    /// eligible bid, because that is the choice a deviation has to answer for.
    pub fn label(&self) -> &'static str {
        match self {
            AwardReason::MenorPreco => "Menor preço entre as propostas habilitadas",
            AwardReason::MenorDuracao => "Viagem mais curta que a da proposta de menor preço",
            AwardReason::MenosConexoes => "Menos conexões que a proposta de menor preço",
            AwardReason::HorarioCompativel => {
                "Horário compatível com o compromisso, que a proposta de menor preço não atende"
            }
            AwardReason::Outro => "Outro motivo (ver justificativa)",
        }
    }
}

/// One bid as the award rule sees it. Itinerary fields are optional because bids
/// recorded before the structured itinerary existed have none.
#[derive(Clone, Debug)]
pub struct Candidate {
    pub proposal_id: Uuid,
    pub price_cents: i64,
    pub duration_minutes: Option<i32>,
    pub connections: Option<i32>,
    /// Supplier active and with every certidão valid at decision time.
    pub eligible: bool,
}

/// Minimum justification when the pick is not the cheapest eligible bid.
pub const DEVIATION_JUSTIFICATION_MIN: usize = 15;
pub const JUSTIFICATION_MIN: usize = 5;

#[derive(Debug, PartialEq, Eq)]
pub struct AwardCheck {
    /// Cheapest eligible bid: the baseline the dossiê compares the winner against.
    pub baseline_proposal_id: Uuid,
    pub baseline_price_cents: i64,
    pub is_lowest_price: bool,
}

/// `candidates` must be in ranking order (price asc, submitted_at asc).
pub fn check_award(
    candidates: &[Candidate],
    winner_id: Uuid,
    reason: AwardReason,
    justification: &str,
) -> Result<AwardCheck, &'static str> {
    let Some(winner) = candidates.iter().find(|c| c.proposal_id == winner_id) else {
        return Err("PROPOSTA_INVALIDA");
    };
    if !winner.eligible {
        return Err("FORNECEDOR_INABILITADO");
    }
    let Some(baseline) = candidates.iter().find(|c| c.eligible) else {
        return Err("FORNECEDOR_INABILITADO");
    };
    let is_lowest_price = winner.price_cents == baseline.price_cents;
    let claim_holds = match reason {
        AwardReason::MenorPreco => is_lowest_price,
        AwardReason::MenorDuracao => matches!(
            (winner.duration_minutes, baseline.duration_minutes),
            (Some(w), Some(b)) if w < b
        ),
        AwardReason::MenosConexoes => matches!(
            (winner.connections, baseline.connections),
            (Some(w), Some(b)) if w < b
        ),
        AwardReason::HorarioCompativel | AwardReason::Outro => true,
    };
    if !claim_holds {
        return Err("MOTIVO_NAO_CONFERE");
    }
    let written = justification.trim().chars().count();
    if written < JUSTIFICATION_MIN {
        return Err("JUSTIFICATIVA_CURTA");
    }
    if !is_lowest_price && written < DEVIATION_JUSTIFICATION_MIN {
        return Err("JUSTIFICATIVA_OBRIGATORIA");
    }
    Ok(AwardCheck {
        baseline_proposal_id: baseline.proposal_id,
        baseline_price_cents: baseline.price_cents,
        is_lowest_price,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn bid(n: u128, price: i64, duration: Option<i32>, connections: Option<i32>, eligible: bool) -> Candidate {
        Candidate {
            proposal_id: Uuid::from_u128(n),
            price_cents: price,
            duration_minutes: duration,
            connections,
            eligible,
        }
    }

    /// Cheapest has a connection and takes 8h50; the 2nd costs R$ 24 more and takes 5h40.
    fn bvb_bsb() -> Vec<Candidate> {
        vec![
            bid(1, 149_900, Some(530), Some(1), true),
            bid(2, 152_300, Some(340), Some(1), true),
            bid(3, 158_000, Some(185), Some(0), true),
        ]
    }

    #[test]
    fn cheapest_eligible_with_lowest_price_reason_passes() {
        let check = check_award(&bvb_bsb(), Uuid::from_u128(1), AwardReason::MenorPreco, "Menor preço.")
            .unwrap();
        assert!(check.is_lowest_price);
        assert_eq!(check.baseline_proposal_id, Uuid::from_u128(1));
    }

    #[test]
    fn lowest_price_claim_on_a_pricier_bid_is_refused() {
        let result = check_award(
            &bvb_bsb(),
            Uuid::from_u128(2),
            AwardReason::MenorPreco,
            "Menor preço entre as propostas válidas.",
        );
        assert_eq!(result, Err("MOTIVO_NAO_CONFERE"));
    }

    #[test]
    fn shorter_trip_justifies_a_pricier_bid_when_explained() {
        let check = check_award(
            &bvb_bsb(),
            Uuid::from_u128(2),
            AwardReason::MenorDuracao,
            "Chega 3h antes; o magistrado tem audiência às 14h.",
        )
        .unwrap();
        assert!(!check.is_lowest_price);
        assert_eq!(check.baseline_price_cents, 149_900);
    }

    #[test]
    fn deviation_needs_a_real_explanation() {
        let result = check_award(&bvb_bsb(), Uuid::from_u128(2), AwardReason::MenorDuracao, "Mais curto");
        assert_eq!(result, Err("JUSTIFICATIVA_OBRIGATORIA"));
    }

    #[test]
    fn shorter_claim_must_actually_be_shorter_than_the_cheapest() {
        let mut bids = bvb_bsb();
        bids[1].duration_minutes = Some(600);
        let result = check_award(
            &bids,
            Uuid::from_u128(2),
            AwardReason::MenorDuracao,
            "Escolhida pelo menor tempo de viagem.",
        );
        assert_eq!(result, Err("MOTIVO_NAO_CONFERE"));
    }

    #[test]
    fn fewer_connections_is_checked_against_the_cheapest() {
        let ok = check_award(
            &bvb_bsb(),
            Uuid::from_u128(3),
            AwardReason::MenosConexoes,
            "Voo direto, passageira com mobilidade reduzida.",
        );
        assert!(ok.is_ok());
        let refused = check_award(
            &bvb_bsb(),
            Uuid::from_u128(2),
            AwardReason::MenosConexoes,
            "Mesma quantidade de conexões, não deveria passar.",
        );
        assert_eq!(refused, Err("MOTIVO_NAO_CONFERE"));
    }

    #[test]
    fn ineligible_supplier_cannot_win() {
        let mut bids = bvb_bsb();
        bids[0].eligible = false;
        let result = check_award(&bids, Uuid::from_u128(1), AwardReason::MenorPreco, "Menor preço.");
        assert_eq!(result, Err("FORNECEDOR_INABILITADO"));
    }

    #[test]
    fn baseline_skips_ineligible_bids() {
        // Cheapest supplier has an expired certidão: the 2nd becomes the lowest
        // eligible price and can be awarded on price alone.
        let mut bids = bvb_bsb();
        bids[0].eligible = false;
        let check = check_award(&bids, Uuid::from_u128(2), AwardReason::MenorPreco, "Menor preço habilitado.")
            .unwrap();
        assert!(check.is_lowest_price);
        assert_eq!(check.baseline_proposal_id, Uuid::from_u128(2));
    }

    #[test]
    fn bids_without_itinerary_cannot_claim_time_or_connections() {
        let bids = vec![bid(1, 100_000, None, None, true), bid(2, 110_000, None, None, true)];
        for reason in [AwardReason::MenorDuracao, AwardReason::MenosConexoes] {
            let result = check_award(&bids, Uuid::from_u128(2), reason, "Justificativa longa o bastante.");
            assert_eq!(result, Err("MOTIVO_NAO_CONFERE"));
        }
    }

    #[test]
    fn free_text_reasons_rely_on_the_justification() {
        let ok = check_award(
            &bvb_bsb(),
            Uuid::from_u128(3),
            AwardReason::HorarioCompativel,
            "Único voo que chega antes da sessão das 9h.",
        );
        assert!(ok.is_ok());
        let short = check_award(&bvb_bsb(), Uuid::from_u128(3), AwardReason::Outro, "Ok");
        assert_eq!(short, Err("JUSTIFICATIVA_CURTA"));
    }

    #[test]
    fn unknown_proposal_is_rejected() {
        let result = check_award(&bvb_bsb(), Uuid::from_u128(9), AwardReason::MenorPreco, "Menor preço.");
        assert_eq!(result, Err("PROPOSTA_INVALIDA"));
    }

    #[test]
    fn reason_codes_round_trip() {
        for reason in [
            AwardReason::MenorPreco,
            AwardReason::MenorDuracao,
            AwardReason::MenosConexoes,
            AwardReason::HorarioCompativel,
            AwardReason::Outro,
        ] {
            assert_eq!(AwardReason::parse(reason.as_str()), Some(reason));
        }
        assert_eq!(AwardReason::parse("menor_preco"), None);
    }
}
