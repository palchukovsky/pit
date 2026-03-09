import openpit
import pytest


@pytest.mark.unit
def test_order_accepts_keyword_arguments_and_numeric_variants() -> None:
    order = openpit.Order(
        underlying_asset="AAPL",
        settlement_asset="USD",
        side="buy",
        quantity="10.5",
        price=185,
    )

    assert order.underlying_asset == "AAPL"
    assert order.settlement_asset == "USD"
    assert order.side == "buy"
    assert order.quantity == "10.5"
    assert order.price == "185"
    assert "Order(" in repr(order)


@pytest.mark.unit
def test_order_rejects_invalid_side() -> None:
    with pytest.raises(ValueError, match="expected 'buy' or 'sell'"):
        openpit.Order(
            underlying_asset="AAPL",
            settlement_asset="USD",
            side="hold",
            quantity=1.0,
            price=10.0,
        )


@pytest.mark.unit
def test_order_rejects_bool_quantity() -> None:
    with pytest.raises(ValueError, match="quantity must be a str, int, or float"):
        openpit.Order(
            underlying_asset="AAPL",
            settlement_asset="USD",
            side="buy",
            quantity=True,
            price=10.0,
        )
