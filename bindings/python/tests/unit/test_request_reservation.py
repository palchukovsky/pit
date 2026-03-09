import openpit
import pytest
from conftest import make_order


@pytest.mark.unit
def test_request_execute_is_single_use() -> None:
    engine = openpit.Engine.builder().build()
    request = engine.start_pre_trade(order=make_order()).request
    first = request.execute()
    assert first.ok
    first.reservation.rollback()

    with pytest.raises(RuntimeError, match="already been executed"):
        request.execute()


@pytest.mark.unit
def test_reservation_finalize_is_single_use() -> None:
    engine = openpit.Engine.builder().build()
    start_result = engine.start_pre_trade(order=make_order())
    reservation = start_result.request.execute().reservation
    reservation.commit()

    with pytest.raises(RuntimeError, match="already been finalized"):
        reservation.rollback()
