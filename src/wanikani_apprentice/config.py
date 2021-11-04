from starlette.config import Config
from starlette.datastructures import Secret

config = Config(".env")

DEBUG = config("DEBUG", cast=bool, default=False)
WANIKANI_API_KEY = config("WANIKANI_API_KEY", cast=Secret)
SESSION_SECRET = config("SESSION_KEY", cast=Secret)
