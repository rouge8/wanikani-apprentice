import pytest
from starlette.testclient import TestClient


@pytest.fixture(autouse=True, scope="session")
def config():
    from starlette.config import environ

    environ["DEBUG"] = "false"
    environ["TRUSTED_HOSTS"] = "testserver"
    environ["HTTPS_ONLY"] = "false"
    environ["WANIKANI_API_KEY"] = "fake-key"
    environ["SESSION_KEY"] = "session-secret"


@pytest.fixture
def anyio_backend():
    return "asyncio"


@pytest.fixture
def test_client():
    """An ASGI test client"""
    from wanikani_apprentice.app import create_app

    app = create_app()
    return TestClient(app)


@pytest.fixture
async def api_client():
    """A WaniKaniAPIClient"""
    from wanikani_apprentice.wanikani import WaniKaniAPIClient

    client = WaniKaniAPIClient("fake-key")
    yield client
    await client.client.aclose()


@pytest.fixture(autouse=True)
def reset_db():
    """Reset the DB between tests"""
    from wanikani_apprentice.db import DB

    DB.radical.clear()
    DB.kanji.clear()
    DB.vocabulary.clear()
