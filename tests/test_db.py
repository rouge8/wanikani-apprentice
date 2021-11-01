import pytest

from wanikani_apprentice.db import Database
from wanikani_apprentice.db import DB
from wanikani_apprentice.db import populate_db
from wanikani_apprentice.models import Kanji
from wanikani_apprentice.models import Radical
from wanikani_apprentice.models import Vocabulary


@pytest.mark.asyncio
async def test_populate_db(faker):
    radicals = [
        Radical(
            id=faker.random_int(),
            document_url=faker.url(),
            characters=faker.pystr(),
            meanings=[],
        )
        for _ in range(faker.random_int(min=1, max=10))
    ]
    kanji = [
        Kanji(
            id=faker.random_int(),
            document_url=faker.url(),
            characters=faker.pystr(),
            meanings=[],
            readings=[],
        )
        for _ in range(faker.random_int(min=1, max=10))
    ]
    vocabulary = [
        Vocabulary(
            id=faker.random_int(),
            document_url=faker.url(),
            characters=faker.pystr(),
            meanings=[],
            readings=[],
        )
        for _ in range(faker.random_int(min=1, max=10))
    ]

    class FakeAPI:
        async def radicals(self):
            for r in radicals:
                yield r

        async def kanji(self):
            for k in kanji:
                yield k

        async def vocabulary(self):
            for v in vocabulary:
                yield v

    await populate_db(FakeAPI())
    assert DB == Database(
        radical={r.id: r for r in radicals},
        kanji={k.id: k for k in kanji},
        vocabulary={v.id: v for v in vocabulary},
    )
