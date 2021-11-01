from functools import partial

from starlette.applications import Starlette

from . import config
from .db import populate_db
from .wanikani import WaniKaniAPIClient


def create_app() -> Starlette:
    # TODO: Use a global httpx.AsyncClient()
    api = WaniKaniAPIClient(config.WANIKANI_API_KEY)

    _populate_db = partial(populate_db, api)

    return Starlette(
        on_startup=[
            _populate_db,
        ],
    )
