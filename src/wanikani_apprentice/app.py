from functools import partial

import httpx
import sentry_sdk
from sentry_sdk.integrations.asgi import SentryAsgiMiddleware
from starlette.applications import Starlette
from starlette.middleware import Middleware
from starlette.middleware.httpsredirect import HTTPSRedirectMiddleware
from starlette.middleware.sessions import SessionMiddleware
from starlette.middleware.trustedhost import TrustedHostMiddleware
from starlette.requests import Request
from starlette.responses import RedirectResponse
from starlette.routing import Route
from starlette.templating import _TemplateResponse

from . import config
from .constants import SESSION_API_KEY
from .db import populate_db
from .models import Kanji
from .models import Radical
from .models import Vocabulary
from .resources import httpx_client
from .resources import templates
from .utils import is_logged_in
from .wanikani import WaniKaniAPIClient


async def index(request: Request) -> RedirectResponse:
    if is_logged_in(request):
        return RedirectResponse(request.url_for("assignments"))
    else:
        return RedirectResponse(request.url_for("login"))


async def login(request: Request) -> _TemplateResponse | RedirectResponse:
    if request.method == "GET":
        if is_logged_in(request):
            return RedirectResponse(request.url_for("assignments"))
        else:
            return templates.TemplateResponse(
                "login.html.j2",
                {"request": request, "invalid_api_key": False},
            )
    elif request.method == "POST":
        form = await request.form()
        api_key = form["api_key"].strip()
        api = WaniKaniAPIClient(api_key, client=httpx_client)

        try:
            await api.username()
        except httpx.HTTPStatusError as err:
            if err.response.status_code == 401:
                return templates.TemplateResponse(
                    "login.html.j2",
                    {"request": request, "invalid_api_key": True},
                    status_code=401,
                )
            else:
                raise err

        request.session[SESSION_API_KEY] = api_key
        return RedirectResponse(request.url_for("assignments"), status_code=303)
    else:
        raise NotImplementedError


async def logout(request: Request) -> RedirectResponse:
    request.session.clear()
    return RedirectResponse(request.url_for("login"))


async def assignments(request: Request) -> _TemplateResponse | RedirectResponse:
    if not is_logged_in(request):
        return RedirectResponse(request.url_for("login"))

    radicals = []
    kanji = []
    vocabulary = []

    api = WaniKaniAPIClient(request.session[SESSION_API_KEY], client=httpx_client)
    async for assignment in api.assignments():
        if isinstance(assignment.subject, Radical):
            radicals.append(assignment)
        elif isinstance(assignment.subject, Kanji):
            kanji.append(assignment)
        elif isinstance(assignment.subject, Vocabulary):
            vocabulary.append(assignment)
        else:
            raise NotImplementedError

    return templates.TemplateResponse(
        "assignments.html.j2",
        {
            "request": request,
            "radicals": radicals,
            "kanji": kanji,
            "vocabulary": vocabulary,
        },
    )


async def test_500(request: Request) -> None:
    1 / 0


def create_app() -> Starlette:
    sentry_sdk.init(
        send_default_pii=True,
        release=config.git_revision(),
    )
    middleware = [
        Middleware(SentryAsgiMiddleware),
    ]

    api = WaniKaniAPIClient(config.WANIKANI_API_KEY, client=httpx_client)

    _populate_db = partial(populate_db, api)

    async def shutdown_client() -> None:  # pragma: no cover
        await httpx_client.aclose()

    middleware.append(
        Middleware(TrustedHostMiddleware, allowed_hosts=config.TRUSTED_HOSTS),
    )
    if config.HTTPS_ONLY:
        middleware.append(Middleware(HTTPSRedirectMiddleware))
    middleware.append(Middleware(SessionMiddleware, secret_key=config.SESSION_KEY))

    return Starlette(
        debug=config.DEBUG,
        on_startup=[
            _populate_db,
        ],
        on_shutdown=[
            shutdown_client,
        ],
        routes=[
            Route("/", index),
            Route("/login", login, methods=["GET", "POST"]),
            Route("/logout", logout),
            Route("/assignments", assignments),
            Route("/test-500", test_500),
        ],
        middleware=middleware,
    )
