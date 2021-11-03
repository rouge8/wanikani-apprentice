import pytest

from wanikani_apprentice.constants import SESSION_API_KEY
from wanikani_apprentice.utils import is_logged_in


@pytest.mark.parametrize(
    "session, expected",
    [
        ({SESSION_API_KEY: "foobar"}, True),
        ({"other-key": "other-value"}, False),
    ],
)
def test_is_logged_in(session, expected, mocker):
    request = mocker.Mock()
    request.session = session

    assert is_logged_in(request) == expected
