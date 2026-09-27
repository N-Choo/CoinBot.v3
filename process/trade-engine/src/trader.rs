use share::models::signals::TradeSignal;

/// This function analyse the results before crating a trade ticket.
///
pub async fn _signal_handler(_signal: &TradeSignal) -> Result<(), share::ProcessError> {
    Ok(())
}
