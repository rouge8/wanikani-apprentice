import ciso8601
from httpx import URL
import pytest

from wanikani_apprentice.db import DB
from wanikani_apprentice.models import Assignment
from wanikani_apprentice.models import Kanji
from wanikani_apprentice.models import Radical
from wanikani_apprentice.models import Vocabulary
from wanikani_apprentice.wanikani import SubjectType


@pytest.mark.asyncio
class TestWaniKaniAPIClient:
    @pytest.fixture
    def headers(self, client):
        return dict(client.client.headers) | {
            "Authorization": f"Bearer {client.api_key}",
        }

    async def test_assignments(self, headers, client, httpx_mock, faker):
        assignments = [
            {
                "id": faker.random_int(),
                "object": "assignment",
                "data": {
                    "subject_id": faker.random_int(),
                    "subject_type": "radical",
                    "srs_stage": faker.random_int(),
                    "available_at": faker.iso8601() + "Z",
                },
            },
            {
                "id": faker.random_int(),
                "object": "assignment",
                "data": {
                    "subject_id": faker.random_int(),
                    "subject_type": "kanji",
                    "srs_stage": faker.random_int(),
                    "available_at": faker.iso8601() + "Z",
                },
            },
            {
                "id": faker.random_int(),
                "object": "assignment",
                "data": {
                    "subject_id": faker.random_int(),
                    "subject_type": "vocabulary",
                    "srs_stage": faker.random_int(),
                    "available_at": faker.iso8601() + "Z",
                },
            },
        ]
        expected_assignments = []
        for assignment in assignments:
            subject_id = assignment["data"]["subject_id"]
            subject_type = assignment["data"]["subject_type"]

            if subject_type == SubjectType.RADICAL:
                subject = Radical(
                    id=subject_id,
                    document_url=faker.url(),
                    characters=faker.pystr(),
                    meanings=[],
                )
                DB.radical[subject_id] = subject
            elif subject_type == SubjectType.KANJI:
                subject = Kanji(
                    id=subject_id,
                    document_url=faker.url(),
                    characters=faker.pystr(),
                    meanings=[],
                    readings=[],
                )
                DB.kanji[subject_id] = subject
            else:
                assert subject_type == SubjectType.VOCABULARY
                subject = Vocabulary(
                    id=subject_id,
                    document_url=faker.url(),
                    characters=faker.pystr(),
                    meanings=[],
                    readings=[],
                )
                DB.vocabulary[subject_id] = subject

            expected_assignments.append(
                Assignment(
                    subject=subject,
                    srs_stage=assignment["data"]["srs_stage"],
                    available_at=ciso8601.parse_datetime(
                        assignment["data"]["available_at"],
                    ),
                ),
            )

        httpx_mock.add_response(
            url=URL(
                f"{client.BASE_URL}/assignments",
                params={"srs_stages": "1,2,3,4", "hidden": "false"},
            ),
            headers=headers,
            json={
                "data": assignments,
            },
        )

        resp = [a async for a in client.assignments()]
        assert resp == expected_assignments

    async def test_radicals(self, headers, client, httpx_mock, faker):
        radicals = [
            {
                "id": faker.random_int(),
                "object": "radical",
                "data": {
                    "document_url": faker.url(),
                    "characters": faker.pystr(),
                    "meanings": [
                        {
                            "meaning": faker.word(),
                            "primary": faker.pybool(),
                            "accepted_answer": faker.pybool(),
                        }
                        for _ in range(faker.random_int(min=1, max=3))
                    ],
                },
            }
            for _ in range(faker.random_int(min=1, max=10))
        ]
        expected_radicals = [
            Radical(
                id=r["id"],
                document_url=r["data"]["document_url"],
                characters=r["data"]["characters"],
                meanings=[
                    meaning["meaning"]
                    for meaning in r["data"]["meanings"]
                    if meaning["accepted_answer"]
                ],
            )
            for r in radicals
        ]

        httpx_mock.add_response(
            url=URL(
                f"{client.BASE_URL}/subjects",
                params={"types": "radical", "hidden": "false"},
            ),
            headers=headers,
            json={
                "pages": {
                    "next_url": None,
                },
                "data": radicals,
            },
        )

        resp = [r async for r in client.radicals()]
        assert resp == expected_radicals

    async def test_kanji(self, headers, client, httpx_mock, faker):
        kanji = [
            {
                "id": faker.random_int(),
                "object": "kanji",
                "data": {
                    "document_url": faker.url(),
                    "characters": faker.pystr(),
                    "meanings": [
                        {
                            "meaning": faker.word(),
                            "primary": faker.pybool(),
                            "accepted_answer": faker.pybool(),
                        }
                        for _ in range(faker.random_int(min=1, max=3))
                    ],
                    "readings": [
                        {
                            "type": faker.word(),
                            "primary": faker.pybool(),
                            "reading": faker.pystr(),
                            "accepted_answer": faker.pybool(),
                        }
                        for _ in range(faker.random_int(min=1, max=5))
                    ],
                },
            }
            for _ in range(faker.random_int(min=3, max=10))
        ]
        expected_kanji = [
            Kanji(
                id=k["id"],
                document_url=k["data"]["document_url"],
                characters=k["data"]["characters"],
                meanings=[
                    meaning["meaning"]
                    for meaning in k["data"]["meanings"]
                    if meaning["accepted_answer"]
                ],
                readings=[
                    reading["reading"]
                    for reading in k["data"]["readings"]
                    if reading["accepted_answer"]
                ],
            )
            for k in kanji
        ]

        httpx_mock.add_response(
            url=URL(
                f"{client.BASE_URL}/subjects",
                params={"types": "kanji", "hidden": "false"},
            ),
            headers=headers,
            json={
                "pages": {
                    "next_url": f"{client.BASE_URL}/subjects?types=kanji&hidden=false&page_after_id=12345",  # noqa: E501
                },
                "data": [kanji[0]],
            },
        )
        httpx_mock.add_response(
            url=URL(
                f"{client.BASE_URL}/subjects",
                params={"types": "kanji", "hidden": "false", "page_after_id": "12345"},
            ),
            headers=headers,
            json={
                "pages": {
                    "next_url": None,
                },
                "data": kanji[1:],
            },
        )

        resp = [k async for k in client.kanji()]
        assert resp == expected_kanji

    async def test_vocabulary(self, headers, client, httpx_mock, faker):
        vocabulary = [
            {
                "id": faker.random_int(),
                "object": "vocabulary",
                "data": {
                    "document_url": faker.url(),
                    "characters": faker.pystr(),
                    "meanings": [
                        {
                            "meaning": faker.word(),
                            "primary": faker.pybool(),
                            "accepted_answer": faker.pybool(),
                        }
                        for _ in range(faker.random_int(min=1, max=3))
                    ],
                    "readings": [
                        {
                            "type": faker.word(),
                            "primary": faker.pybool(),
                            "reading": faker.pystr(),
                            "accepted_answer": faker.pybool(),
                        }
                        for _ in range(faker.random_int(min=1, max=2))
                    ],
                },
            }
            for _ in range(faker.random_int(min=3, max=10))
        ]
        expected_vocabulary = [
            Vocabulary(
                id=v["id"],
                document_url=v["data"]["document_url"],
                characters=v["data"]["characters"],
                meanings=[
                    meaning["meaning"]
                    for meaning in v["data"]["meanings"]
                    if meaning["accepted_answer"]
                ],
                readings=[
                    reading["reading"]
                    for reading in v["data"]["readings"]
                    if reading["accepted_answer"]
                ],
            )
            for v in vocabulary
        ]

        httpx_mock.add_response(
            url=URL(
                f"{client.BASE_URL}/subjects",
                params={"types": "vocabulary", "hidden": "false"},
            ),
            headers=headers,
            json={
                "pages": {
                    "next_url": f"{client.BASE_URL}/subjects?types=kanji&hidden=false&page_after_id=987",  # noqa: E501
                },
                "data": [vocabulary[0]],
            },
        )
        httpx_mock.add_response(
            url=URL(
                f"{client.BASE_URL}/subjects",
                params={
                    "types": "vocabulary",
                    "hidden": "false",
                    "page_after_id": "987",
                },
            ),
            headers=headers,
            json={
                "pages": {
                    "next_url": f"{client.BASE_URL}/subjects?types=kanji&hidden=false&page_after_id=1234",  # noqa: E501
                },
                "data": [vocabulary[1]],
            },
        )
        httpx_mock.add_response(
            url=URL(
                f"{client.BASE_URL}/subjects",
                params={
                    "types": "vocabulary",
                    "hidden": "false",
                    "page_after_id": "1234",
                },
            ),
            headers=headers,
            json={
                "pages": {
                    "next_url": None,
                },
                "data": vocabulary[2:],
            },
        )

        resp = [v async for v in client.vocabulary()]
        assert resp == expected_vocabulary
