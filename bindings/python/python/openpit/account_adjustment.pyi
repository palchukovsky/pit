from __future__ import annotations

from .core import Instrument
from .param import (
    AdjustmentAmount,
    Asset,
    Leverage,
    PositionMode,
    PositionSize,
    Price,
)

class Amount:
    """Grouped total/reserved/pending adjustment payload."""

    def __init__(
        self,
        *,
        total: AdjustmentAmount | None = None,
        reserved: AdjustmentAmount | None = None,
        pending: AdjustmentAmount | None = None,
    ) -> None: ...
    @property
    def total(self) -> AdjustmentAmount | None:
        """Actual resulting balance/position value."""

    @property
    def reserved(self) -> AdjustmentAmount | None:
        """Amount earmarked for outgoing settlement.

        Unavailable for immediate use.
        """

    @property
    def pending(self) -> AdjustmentAmount | None:
        """Amount in-flight for incoming acquisition and not yet finalized."""

class BalanceOperation:
    """Direct physical balance adjustment."""

    def __init__(
        self,
        *,
        asset: Asset,
        average_entry_price: Price | None = None,
    ) -> None: ...
    @property
    def asset(self) -> Asset: ...
    @property
    def average_entry_price(self) -> Price | None:
        """Optional cost basis for the adjusted physical balance."""

class PositionOperation:
    """Direct derivatives-like position adjustment."""

    def __init__(
        self,
        *,
        instrument: Instrument,
        collateral_asset: Asset,
        average_entry_price: Price,
        mode: PositionMode,
        leverage: Leverage | int | float | None = None,
    ) -> None: ...
    @property
    def instrument(self) -> Instrument: ...
    @property
    def collateral_asset(self) -> Asset: ...
    @property
    def average_entry_price(self) -> Price:
        """Average entry price for the adjusted position state."""

    @property
    def mode(self) -> PositionMode:
        """Netting vs hedged position representation."""

    @property
    def leverage(self) -> Leverage | None:
        """Optional leverage snapshot/setting carried with the position adjustment."""

class Bounds:
    """Optional post-adjustment inclusive limits."""

    def __init__(
        self,
        *,
        total_upper: PositionSize | None = None,
        total_lower: PositionSize | None = None,
        reserved_upper: PositionSize | None = None,
        reserved_lower: PositionSize | None = None,
        pending_upper: PositionSize | None = None,
        pending_lower: PositionSize | None = None,
    ) -> None: ...
    @property
    def total_upper(self) -> PositionSize | None:
        """Allowed post-adjustment inclusive upper bound for total."""

    @property
    def total_lower(self) -> PositionSize | None:
        """Allowed post-adjustment inclusive lower bound for total."""

    @property
    def reserved_upper(self) -> PositionSize | None:
        """Allowed post-adjustment inclusive upper bound for reserved."""

    @property
    def reserved_lower(self) -> PositionSize | None:
        """Allowed post-adjustment inclusive lower bound for reserved."""

    @property
    def pending_upper(self) -> PositionSize | None:
        """Allowed post-adjustment inclusive upper bound for pending."""

    @property
    def pending_lower(self) -> PositionSize | None:
        """Allowed post-adjustment inclusive lower bound for pending."""

class Adjustment:
    """Extensible non-trading account-adjustment model."""

    def __init__(
        self,
        *,
        operation: BalanceOperation | PositionOperation | None = None,
        amount: Amount | None = None,
        bounds: Bounds | None = None,
    ) -> None: ...
    @property
    def operation(
        self,
    ) -> BalanceOperation | PositionOperation | None: ...
    @property
    def amount(self) -> Amount | None: ...
    @property
    def bounds(self) -> Bounds | None: ...

__all__ = [
    "Adjustment",
    "Amount",
    "BalanceOperation",
    "Bounds",
    "PositionOperation",
]
