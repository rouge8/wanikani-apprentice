from collections.abc import AsyncIterable
import enum
import typing

import attr
import httpx

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
        self.client.headers.update(
            {
                "Authorization": f"Bearer {self.api_key}",
                "Wanikani-Revision": "20170710",
            },
        )

    async def assignments(self) -> list[dict[str, typing.Any]]:
        """Get all Apprentice assignments"""
        # TODO: Handle possible (but unlikely) pagination
        resp = await self.client.get(
            f"{self.BASE_URL}/assignments",
            params={
                "srs_stages": ",".join(str(stage) for stage in APPRENTICE_SRS_STAGES),
                "hidden": "false",
            },
        )
        return resp.json()["data"]  # type: ignore[no-any-return]

    async def _subjects(
        self,
        subject_type: SubjectType,
    ) -> AsyncIterable[dict[str, typing.Any]]:
        next_url = f"{self.BASE_URL}/subjects"

        while next_url is not None:
            resp = await self.client.get(
                next_url,
                params={"types": subject_type.value, "hidden": "false"},
            )
            resp = resp.json()

            next_url = resp["pages"]["next_url"]

            for subject in resp["data"]:
                yield subject

    async def radicals(self) -> AsyncIterable[dict[str, typing.Any]]:
        """Get all radicals"""
        return self._subjects(SubjectType.RADICAL)

    async def kanji(self) -> AsyncIterable[dict[str, typing.Any]]:
        """Get all kanji"""
        return self._subjects(SubjectType.KANJI)

    async def vocabulary(self) -> AsyncIterable[dict[str, typing.Any]]:
        """Get all vocabulary"""
        return self._subjects(SubjectType.VOCABULARY)
