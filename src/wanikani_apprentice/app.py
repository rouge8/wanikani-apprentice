from functools import partial
import os.path
import typing

from starlette.applications import Starlette
from starlette.routing import Route
from starlette.templating import Jinja2Templates

if typing.TYPE_CHECKING:
    from starlette.requests import Request
    from starlette.templating import _TemplateResponse

from . import config
from .db import populate_db
from .wanikani import WaniKaniAPIClient

HERE = os.path.dirname(__file__)

templates = Jinja2Templates(directory=os.path.join(HERE, "templates"))


async def login(request: "Request") -> "_TemplateResponse":
    return templates.TemplateResponse("login.html.j2", {"request": request})


async def assignments(request: "Request") -> "_TemplateResponse":
    return templates.TemplateResponse("assignments.html.j2", {"request": request})


def create_app() -> Starlette:
    # TODO: Use a global httpx.AsyncClient()
    api = WaniKaniAPIClient(config.WANIKANI_API_KEY)

    _populate_db = partial(populate_db, api)

    return Starlette(
        # TODO: Remove
        debug=True,
        on_startup=[
            _populate_db,
        ],
        routes=[
            Route("/login", login),
            Route("/assignments", assignments),
        ],
    )
