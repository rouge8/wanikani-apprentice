import enum
import typing

import attr
import requests

APPRENTICE_SRS_STAGES = [1, 2, 3, 4]


class SubjectType(str, enum.Enum):
    RADICAL = "radical"
    KANJI = "kanji"
    VOCABULARY = "vocabulary"


@attr.frozen
class WaniKaniAPIClient:
    BASE_URL = "https://api.wanikani.com/v2"

    api_key: str = attr.field(repr=False)
    session: requests.Session = attr.field(factory=requests.Session)

    def __attrs_post_init__(self):
        self.session.headers.update(
            {
                "Authorization": f"Bearer {self.api_key}",
                "Wanikani-Revision": "20170710",
            },
        )

    def assignments(self) -> list[dict]:
        """Get all Apprentice assignments"""
        # TODO: Handle possible (but unlikely) pagination
        return self.session.get(
            f"{self.BASE_URL}/assignments",
            params={
                "srs_stages": ",".join(str(stage) for stage in APPRENTICE_SRS_STAGES),
                "hidden": "false",
            },
        ).json()["data"]

    def _subjects(self, subject_type: SubjectType) -> typing.Iterator[dict]:
        next_url = f"{self.BASE_URL}/subjects"

        while next_url is not None:
            resp = self.session.get(
                next_url,
                params={"types": subject_type, "hidden": "false"},
            ).json()

            next_url = resp["pages"]["next_url"]

            yield from resp["data"]

    def radicals(self) -> list[dict]:
        """Get all radicals"""
        return self._subjects(SubjectType.RADICAL)

    def kanji(self) -> typing.Iterator[dict]:
        """Get all kanji"""
        return self._subjects(SubjectType.KANJI)

    def vocabulary(self) -> typing.Iterator[dict]:
        """Get all vocabulary"""
        return self._subjects(SubjectType.VOCABULARY)
