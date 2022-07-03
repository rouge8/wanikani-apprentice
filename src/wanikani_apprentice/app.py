from datetime import datetime
from functools import partial
import operator
import os.path

import attrs
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
from starlette.responses import Response
from starlette.routing import Mount
from starlette.routing import Route
from starlette.staticfiles import StaticFiles
from starlette.templating import _TemplateResponse
from starlette.types import ASGIApp
from starlette.types import Receive
from starlette.types import Scope
from starlette.types import Send
import structlog
from typing_extensions import assert_never

from . import config
from .constants import BS_PRIMARY_COLOR
from .constants import HERE
from .constants import SESSION_API_KEY
from .db import populate_db
from .models import Kanji
from .models import Radical
from .models import Vocabulary
from .resources import httpx_client
from .resources import templates
from .utils import is_logged_in
from .wanikani import WaniKaniAPIClient

log = structlog.get_logger()


@attrs.frozen
class LBHeartbeatMiddleware:
    app: ASGIApp

    async def __call__(self, scope: Scope, receive: Receive, send: Send) -> None:
        if scope["type"] == "http" and scope["path"] == "/__lbheartbeat__":
            response = Response("OK")
            await response(scope, receive, send)
        else:
            await self.app(scope, receive, send)


async def index(request: Request) -> RedirectResponse:
    if is_logged_in(request):
        return RedirectResponse(request.url_for("assignments"))
    else:
        return RedirectResponse(request.url_for("login"))


async def login(request: Request) -> _TemplateResponse | RedirectResponse | Response:
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
        api_key = form["api_key"]
        assert isinstance(api_key, str)
        api_key = api_key.strip()
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
    elif request.method == "HEAD":
        return Response()
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

    assignments = [assignment async for assignment in api.assignments()]
    # Sort assignments by time until next review, soonest available first
    assignments.sort(key=operator.attrgetter("available_at"))

    for assignment in assignments:
        if isinstance(assignment.subject, Radical):
            radicals.append(assignment)
        elif isinstance(assignment.subject, Kanji):
            kanji.append(assignment)
        elif isinstance(assignment.subject, Vocabulary):
            vocabulary.append(assignment)
        else:
            assert_never(assignment.subject)

    return templates.TemplateResponse(
        "assignments.html.j2",
        {
            "request": request,
            "radicals": radicals,
            "kanji": kanji,
            "vocabulary": vocabulary,
            "now": datetime.now().astimezone(),
        },
    )


async def radical_svg(request: Request) -> Response:
    """
    Mirror the WaniKani radical SVGs, replacing the ``stroke`` color with our
    primary color.

    This is necessary because browsers attempt to download the SVGs from their
    CDN instead of render them.
    """
    url = f"https://files.wanikani.com/{request.path_params['path']}"
    log.info("downloading SVG", url=url)
    resp = await httpx_client.get(url)
    resp.raise_for_status()
    svg = resp.text.replace("stroke:#000", f"stroke:{BS_PRIMARY_COLOR}")
    return Response(svg, media_type="image/svg+xml")


async def test_500(request: Request) -> None:
    1 / 0


def create_app() -> Starlette:
    sentry_sdk.init(
        send_default_pii=True,
        release=config.git_revision(),
    )
    middleware = [
        Middleware(SentryAsgiMiddleware),
        Middleware(LBHeartbeatMiddleware),
    ]

    api = WaniKaniAPIClient(str(config.WANIKANI_API_KEY), client=httpx_client)

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
            Route("/radical-svg/{path}", radical_svg),
            Route("/test-500", test_500),
            Mount(
                "/static",
                app=StaticFiles(directory=os.path.join(HERE, "static")),
                name="static",
            ),
        ],
        middleware=middleware,
    )
