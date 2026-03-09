import openpit
import pytest
from conftest import make_order


class AcceptPolicy(openpit.pretrade.Policy):
    @property
    def name(self) -> str:
        return "AcceptPolicy"

    def perform_pre_trade_check(
        self, *, context: openpit.pretrade.PolicyContext
    ) -> openpit.pretrade.PolicyDecision:
        _ = context
        return openpit.pretrade.PolicyDecision.accept()

    def apply_execution_report(
        self,
        *,
        report: openpit.pretrade.ExecutionReport,
    ) -> bool:
        _ = report
        return False


class NamedRejectPolicy(openpit.pretrade.Policy):
    def __init__(self, *, policy_name: str) -> None:
        self._policy_name = policy_name

    @property
    def name(self) -> str:
        return self._policy_name

    def perform_pre_trade_check(
        self, *, context: openpit.pretrade.PolicyContext
    ) -> openpit.pretrade.PolicyDecision:
        _ = context
        return openpit.pretrade.PolicyDecision.accept()

    def apply_execution_report(
        self,
        *,
        report: openpit.pretrade.ExecutionReport,
    ) -> bool:
        _ = report
        return False


@pytest.mark.unit
def test_engine_builder_supports_chaining_and_main_stage_policy() -> None:
    engine = (
        openpit.Engine.builder()
        .check_pre_trade_start_policy(
            policy=openpit.pretrade.policies.OrderValidationPolicy(),
        )
        .pre_trade_policy(policy=AcceptPolicy())
        .build()
    )

    start_result = engine.start_pre_trade(order=make_order())
    assert start_result.ok
    execute_result = start_result.request.execute()
    assert execute_result.ok
    execute_result.reservation.rollback()


@pytest.mark.unit
def test_builder_rejects_duplicate_policy_names() -> None:
    with pytest.raises(ValueError, match="duplicate policy name"):
        (
            openpit.Engine.builder()
            .pre_trade_policy(policy=NamedRejectPolicy(policy_name="dup"))
            .pre_trade_policy(policy=NamedRejectPolicy(policy_name="dup"))
            .build()
        )
