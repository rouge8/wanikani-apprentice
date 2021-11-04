import subprocess

from starlette.config import Config
from starlette.datastructures import CommaSeparatedStrings
from starlette.datastructures import Secret

config = Config(".env")

DEBUG = config("DEBUG", cast=bool, default=False)
TRUSTED_HOSTS = config("TRUSTED_HOSTS", cast=CommaSeparatedStrings)
HTTPS_ONLY = config("HTTPS_ONLY", cast=bool, default=True)

WANIKANI_API_KEY = config("WANIKANI_API_KEY", cast=Secret)
SESSION_SECRET = config("SESSION_KEY", cast=Secret)

SENTRY_ENABLED = config("SENTRY_ENABLED", cast=bool, default=False)


def git_revision() -> str:
    return subprocess.check_output(["git", "rev-parse", "HEAD"]).decode("ascii")
