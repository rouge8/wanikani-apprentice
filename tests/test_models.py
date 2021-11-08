from datetime import datetime
from datetime import timedelta

import pytest

from .factories import AssignmentFactory


class TestAssignment:
    @pytest.mark.parametrize(
        "offset, expected",
        [
            (timedelta(0), "now"),
            (timedelta(seconds=-1), "now"),
            (timedelta(minutes=55), "in 1 hour"),
            (timedelta(hours=23), "in 1 day"),
            (timedelta(minutes=90), "in 2 hours"),
            (timedelta(minutes=20), "in 20 minutes"),
        ],
    )
    def test_display_time_remaining(self, offset, expected):
        assignment = AssignmentFactory.build(available_at=datetime.now())

        now = assignment.available_at - offset

        assert assignment.display_time_remaining(now) == expected
