import pygit2
from starlette.config import Config
from starlette.datastructures import CommaSeparatedStrings
from starlette.datastructures import Secret

config = Config(".env")

DEBUG = config("DEBUG", cast=bool, default=False)
TRUSTED_HOSTS = config("TRUSTED_HOSTS", cast=CommaSeparatedStrings)
HTTPS_ONLY = config("HTTPS_ONLY", cast=bool, default=True)

WANIKANI_API_KEY = config("WANIKANI_API_KEY", cast=Secret)
SESSION_KEY = config("SESSION_KEY", cast=Secret)


def git_revision() -> str:
    repo = pygit2.Repository(".")
    head = repo.revparse_single("HEAD")
    return head.hex  # type: ignore
