from httpx import URL
import pytest

from wanikani_apprentice.wanikani import WaniKaniAPIClient


@pytest.mark.asyncio
class TestWaniKaniAPIClient:
    @pytest.fixture
    async def client(self):
        client = WaniKaniAPIClient("fake-key")
        yield client
        await client.client.aclose()

    async def test_assignments(self, client, httpx_mock, faker):
        expected_assignments = [
            {
                "id": faker.random_int(),
                "object": "assignment",
                "data": {
                    "subject_id": faker.random_int(),
                    "subject_type": faker.random_element(
                        ["radical", "kanji", "vocabulary"],
                    ),
                    "srs_stage": faker.random_int(),
                },
            }
            for _ in range(faker.random_int(min=1, max=10))
        ]

        httpx_mock.add_response(
            url=URL(
                f"{client.BASE_URL}/assignments",
                params={"srs_stages": "1,2,3,4", "hidden": "false"},
            ),
            headers=client.client.headers,
            json={
                "data": expected_assignments,
            },
        )

        resp = await client.assignments()
        assert resp == expected_assignments

    async def test_radicals(self, client, httpx_mock, faker):
        expected_radicals = [
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

        httpx_mock.add_response(
            url=URL(
                f"{client.BASE_URL}/subjects",
                params={"types": "radical", "hidden": "false"},
            ),
            headers=client.client.headers,
            json={
                "pages": {
                    "next_url": None,
                },
                "data": expected_radicals,
            },
        )

        resp = [r async for r in await client.radicals()]
        assert resp == expected_radicals

    async def test_kanji(self, client, httpx_mock, faker):
        expected_kanji = [
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

        httpx_mock.add_response(
            url=URL(
                f"{client.BASE_URL}/subjects",
                params={"types": "kanji", "hidden": "false"},
            ),
            headers=client.client.headers,
            json={
                "pages": {
                    "next_url": f"{client.BASE_URL}/subjects?types=kanji&hidden=false&page_after_id=12345",  # noqa: E501
                },
                "data": [expected_kanji[0]],
            },
        )
        httpx_mock.add_response(
            url=URL(
                f"{client.BASE_URL}/subjects",
                params={"types": "kanji", "hidden": "false", "page_after_id": "12345"},
            ),
            headers=client.client.headers,
            json={
                "pages": {
                    "next_url": None,
                },
                "data": expected_kanji[1:],
            },
        )

        resp = [k async for k in await client.kanji()]
        assert resp == expected_kanji

    async def test_vocabulary(self, client, httpx_mock, faker):
        expected_vocabulary = [
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

        httpx_mock.add_response(
            url=URL(
                f"{client.BASE_URL}/subjects",
                params={"types": "vocabulary", "hidden": "false"},
            ),
            headers=client.client.headers,
            json={
                "pages": {
                    "next_url": f"{client.BASE_URL}/subjects?types=kanji&hidden=false&page_after_id=987",  # noqa: E501
                },
                "data": [expected_vocabulary[0]],
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
            headers=client.client.headers,
            json={
                "pages": {
                    "next_url": f"{client.BASE_URL}/subjects?types=kanji&hidden=false&page_after_id=1234",  # noqa: E501
                },
                "data": [expected_vocabulary[1]],
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
            headers=client.client.headers,
            json={
                "pages": {
                    "next_url": None,
                },
                "data": expected_vocabulary[2:],
            },
        )

        resp = [v async for v in await client.vocabulary()]
        assert resp == expected_vocabulary
