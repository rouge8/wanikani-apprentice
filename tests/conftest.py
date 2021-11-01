import pytest


@pytest.fixture
async def client():
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
