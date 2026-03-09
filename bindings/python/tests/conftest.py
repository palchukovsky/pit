import openpit


def make_order(
    *,
    side: str = "buy",
    quantity: float = 1.0,
    price: float = 10.0,
    underlying_asset: str = "AAPL",
    settlement_asset: str = "USD",
) -> openpit.Order:
    return openpit.Order(
        underlying_asset=underlying_asset,
        settlement_asset=settlement_asset,
        side=side,
        quantity=quantity,
        price=price,
    )


def make_report(
    *,
    pnl: float,
    fee: float = 0.0,
    underlying_asset: str = "AAPL",
    settlement_asset: str = "USD",
) -> openpit.pretrade.ExecutionReport:
    return openpit.pretrade.ExecutionReport(
        underlying_asset=underlying_asset,
        settlement_asset=settlement_asset,
        pnl=pnl,
        fee=fee,
    )
