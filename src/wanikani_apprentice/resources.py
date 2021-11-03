import os.path

import httpx
from starlette.templating import Jinja2Templates

from .utils import is_logged_in

HERE = os.path.dirname(__file__)

templates = Jinja2Templates(directory=os.path.join(HERE, "templates"))
templates.env.filters["is_logged_in"] = is_logged_in

httpx_client = httpx.AsyncClient()
