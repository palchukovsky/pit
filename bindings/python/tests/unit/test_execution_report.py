import openpit
import pytest


@pytest.mark.unit
def test_execution_report_defaults_fee_and_exposes_fields() -> None:
    report = openpit.pretrade.ExecutionReport(
        underlying_asset="AAPL",
        settlement_asset="USD",
        pnl=-5.0,
    )

    assert report.underlying_asset == "AAPL"
    assert report.settlement_asset == "USD"
    assert report.pnl == "-5"
    assert report.fee == "0"
    assert "ExecutionReport(" in repr(report)


@pytest.mark.unit
def test_execution_report_rejects_invalid_asset() -> None:
    with pytest.raises(ValueError):
        openpit.pretrade.ExecutionReport(
            underlying_asset="AAPL",
            settlement_asset="",
            pnl=1.0,
            fee=0.1,
        )
