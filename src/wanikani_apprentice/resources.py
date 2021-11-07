import os.path

import httpx
from starlette.templating import Jinja2Templates

from .constants import HERE
from .utils import is_logged_in

templates = Jinja2Templates(directory=os.path.join(HERE, "templates"))
templates.env.filters["is_logged_in"] = is_logged_in

httpx_client = httpx.AsyncClient()
