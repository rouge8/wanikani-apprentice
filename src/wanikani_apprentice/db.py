import typing

import attr

from .models import Kanji
from .models import Radical
from .models import Vocabulary

if typing.TYPE_CHECKING:
    from .wanikani import WaniKaniAPIClient


@attr.frozen
class Database:
    radical: dict[int, Radical] = attr.field(factory=dict)
    kanji: dict[int, Kanji] = attr.field(factory=dict)
    vocabulary: dict[int, Vocabulary] = attr.field(factory=dict)


DB = Database()


async def populate_db(api: "WaniKaniAPIClient") -> None:
    async for radical in api.radicals():
        DB.radical[radical.id] = radical
    async for kanji in api.kanji():
        DB.kanji[kanji.id] = kanji
    async for vocabulary in api.vocabulary():
        DB.vocabulary[vocabulary.id] = vocabulary
