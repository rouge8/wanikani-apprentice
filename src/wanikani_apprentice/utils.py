from starlette.requests import Request

from .constants import SESSION_API_KEY


def is_logged_in(request: Request) -> bool:
    return request.session.get(SESSION_API_KEY) is not None
