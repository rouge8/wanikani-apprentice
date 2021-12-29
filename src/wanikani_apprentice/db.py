import typing

import anyio
import attrs
import structlog

from .models import Kanji
from .models import Radical
from .models import Vocabulary

if typing.TYPE_CHECKING:
    from .wanikani import WaniKaniAPIClient


log = structlog.get_logger()


@attrs.frozen
class Database:
    radical: dict[int, Radical] = attrs.field(factory=dict)
    kanji: dict[int, Kanji] = attrs.field(factory=dict)
    vocabulary: dict[int, Vocabulary] = attrs.field(factory=dict)


DB = Database()


async def populate_db(api: "WaniKaniAPIClient") -> None:
    async with anyio.create_task_group() as tg:
        tg.start_soon(_populate_radicals, api)
        tg.start_soon(_populate_kanji, api)
        tg.start_soon(_populate_vocabulary, api)


async def _populate_radicals(api: "WaniKaniAPIClient") -> None:
    async for radical in api.radicals():
        DB.radical[radical.id] = radical
    log.info("loaded radicals", n=len(DB.radical))


async def _populate_kanji(api: "WaniKaniAPIClient") -> None:
    async for kanji in api.kanji():
        DB.kanji[kanji.id] = kanji
    log.info("loaded kanji", n=len(DB.kanji))


async def _populate_vocabulary(api: "WaniKaniAPIClient") -> None:
    async for vocabulary in api.vocabulary():
        DB.vocabulary[vocabulary.id] = vocabulary
    log.info("loaded vocabulary", n=len(DB.vocabulary))
