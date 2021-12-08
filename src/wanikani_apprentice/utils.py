from typing import NoReturn

from starlette.requests import Request

from .constants import SESSION_API_KEY


def is_logged_in(request: Request) -> bool:
    return request.session.get(SESSION_API_KEY) is not None


def assert_never(value: NoReturn) -> NoReturn:
    """A hack to enable exhaustiveness checking in mypy."""
    assert False, f"Unhandled value: {value} ({type(value).__name__})"  # noqa: B011
