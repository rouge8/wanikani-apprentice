import pytest

from wanikani_apprentice.db import Database
from wanikani_apprentice.db import DB
from wanikani_apprentice.db import populate_db

from .factories import KanjiFactory
from .factories import RadicalFactory
from .factories import VocabularyFactory


@pytest.mark.anyio
async def test_populate_db(faker):
    radicals = RadicalFactory.build_batch(faker.random_int(min=1, max=10))
    kanji = KanjiFactory.build_batch(faker.random_int(min=1, max=10))
    vocabulary = VocabularyFactory.build_batch(faker.random_int(min=1, max=10))

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
