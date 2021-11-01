from collections.abc import AsyncIterable
import enum
import time
import typing

import attr
import ciso8601
import httpx
import structlog

from .db import DB
from .models import Assignment
from .models import Kanji
from .models import Radical
from .models import Vocabulary

log = structlog.get_logger()

APPRENTICE_SRS_STAGES = [1, 2, 3, 4]


class SubjectType(str, enum.Enum):
    RADICAL = "radical"
    KANJI = "kanji"
    VOCABULARY = "vocabulary"


@attr.frozen
class WaniKaniAPIClient:
    BASE_URL = "https://api.wanikani.com/v2"

    api_key: str = attr.field(repr=False)
    client: httpx.AsyncClient = attr.field(factory=httpx.AsyncClient)

    def __attrs_post_init__(self) -> None:
        self.client.headers.update({"Wanikani-Revision": "20170710"})

    async def _request(self, path: str, params: dict[str, str]) -> httpx.Response:
        log.info("requesting", path=path, params=params)
        start = time.time()
        resp = await self.client.get(
            f"{self.BASE_URL}/{path}",
            params=params,
            headers={"Authorization": f"Bearer {self.api_key}"},
        )
        end = time.time()
        log.info(
            "requested",
            path=path,
            params=params,
            status_code=resp.status_code,
            duration=end - start,
        )
        return resp

    async def assignments(self) -> AsyncIterable[Assignment]:
        """Get all Apprentice assignments"""
        # TODO: Handle possible (but unlikely) pagination
        resp = await self._request(
            "assignments",
            {
                "srs_stages": ",".join(str(stage) for stage in APPRENTICE_SRS_STAGES),
                "hidden": "false",
            },
        )
        for assignment in resp.json()["data"]:
            subject_id = assignment["data"]["subject_id"]
            subject_type = assignment["data"]["subject_type"]

            if subject_type == SubjectType.RADICAL:
                subject = DB.radical[subject_id]
            elif subject_type == SubjectType.KANJI:
                subject = DB.kanji[subject_id]  # type: ignore[assignment]
            elif subject_type == SubjectType.VOCABULARY:
                subject = DB.vocabulary[subject_id]  # type: ignore[assignment]
            else:
                raise NotImplementedError

            yield Assignment(
                subject=subject,
                srs_stage=assignment["data"]["srs_stage"],
                available_at=ciso8601.parse_datetime(
                    assignment["data"]["available_at"],
                ),
            )

    async def _subjects(
        self,
        subject_type: SubjectType,
    ) -> AsyncIterable[dict[str, typing.Any]]:
        next_url = "subjects"

        while next_url is not None:
            resp = await self._request(
                next_url,
                {"types": subject_type.value, "hidden": "false"},
            )
            resp = resp.json()

            next_url = resp["pages"]["next_url"]
            if next_url is not None:
                next_url = next_url.split(f"{self.BASE_URL}/", 1)[1]

            for subject in resp["data"]:
                yield subject

    async def radicals(self) -> AsyncIterable[Radical]:
        """Get all radicals"""
        async for radical in self._subjects(SubjectType.RADICAL):
            yield Radical(
                id=radical["id"],
                document_url=radical["data"]["document_url"],
                characters=radical["data"]["characters"],
                meanings=[
                    meaning["meaning"]
                    for meaning in radical["data"]["meanings"]
                    if meaning["accepted_answer"]
                ],
            )

    async def kanji(self) -> AsyncIterable[Kanji]:
        """Get all kanji"""
        async for kanji in self._subjects(SubjectType.KANJI):
            yield Kanji(
                id=kanji["id"],
                document_url=kanji["data"]["document_url"],
                characters=kanji["data"]["characters"],
                meanings=[
                    meaning["meaning"]
                    for meaning in kanji["data"]["meanings"]
                    if meaning["accepted_answer"]
                ],
                readings=[
                    reading["reading"]
                    for reading in kanji["data"]["readings"]
                    if reading["accepted_answer"]
                ],
            )

    async def vocabulary(self) -> AsyncIterable[Vocabulary]:
        """Get all vocabulary"""
        async for vocab in self._subjects(SubjectType.VOCABULARY):
            yield Vocabulary(
                id=vocab["id"],
                document_url=vocab["data"]["document_url"],
                characters=vocab["data"]["characters"],
                meanings=[
                    meaning["meaning"]
                    for meaning in vocab["data"]["meanings"]
                    if meaning["accepted_answer"]
                ],
                readings=[
                    reading["reading"]
                    for reading in vocab["data"]["readings"]
                    if reading["accepted_answer"]
                ],
            )
