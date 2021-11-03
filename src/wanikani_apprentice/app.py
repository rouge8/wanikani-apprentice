from functools import partial
import os.path

from starlette.applications import Starlette
from starlette.middleware import Middleware
from starlette.middleware.sessions import SessionMiddleware
from starlette.requests import Request
from starlette.responses import RedirectResponse
from starlette.routing import Route
from starlette.templating import _TemplateResponse
from starlette.templating import Jinja2Templates

from . import config
from .constants import SESSION_API_KEY
from .db import populate_db
from .models import Kanji
from .models import Radical
from .models import Vocabulary
from .utils import is_logged_in
from .wanikani import WaniKaniAPIClient

HERE = os.path.dirname(__file__)

templates = Jinja2Templates(directory=os.path.join(HERE, "templates"))
templates.env.filters["is_logged_in"] = is_logged_in


async def login(request: Request) -> _TemplateResponse | RedirectResponse:
    if request.method == "GET":
        if is_logged_in(request):
            return RedirectResponse(request.url_for("assignments"))
        else:
            return templates.TemplateResponse("login.html.j2", {"request": request})
    elif request.method == "POST":
        form = await request.form()
        # TODO: Get /user to validate the API key
        request.session[SESSION_API_KEY] = form["api_key"].strip()
        return RedirectResponse(request.url_for("assignments"), status_code=303)
    else:
        raise NotImplementedError


async def logout(request: Request) -> RedirectResponse:
    request.session.clear()
    return RedirectResponse(request.url_for("login"))


async def assignments(request: Request) -> _TemplateResponse:
    radicals = []
    kanji = []
    vocabulary = []

    api = WaniKaniAPIClient(request.session[SESSION_API_KEY])
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


def create_app() -> Starlette:
    # TODO: Use a global httpx.AsyncClient()
    api = WaniKaniAPIClient(config.WANIKANI_API_KEY)

    _populate_db = partial(populate_db, api)

    return Starlette(
        debug=config.DEBUG,
        on_startup=[
            _populate_db,
        ],
        routes=[
            Route("/login", login, methods=["GET", "POST"]),
            Route("/logout", logout),
            Route("/assignments", assignments),
        ],
        middleware=[
            # TODO: HTTPSRedirectMiddleware
            # TODO: TrustedHostMiddleware
            Middleware(SessionMiddleware, secret_key=config.SESSION_SECRET),
        ],
    )
