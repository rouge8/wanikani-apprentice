from collections.abc import AsyncIterable
import operator

import attrs
import httpx
import pytest
from starlette.testclient import TestClient

from wanikani_apprentice.constants import BS_PRIMARY_COLOR
from wanikani_apprentice.models import Assignment

from .factories import AssignmentFactory
from .factories import KanjiFactory
from .factories import RadicalFactory
from .factories import VocabularyFactory


@attrs.define(slots=False)
class FakeAPI:
    api_key: str | None = attrs.field(init=False, default=None)
    client: httpx.AsyncClient | None = attrs.field(init=False, default=None)

    _username: str | None = attrs.field(init=False, default=None)
    _assignments: list[Assignment] = attrs.field(init=False, factory=list)

    def __call__(self, api_key: str, client: httpx.AsyncClient) -> "FakeAPI":
        self.api_key = api_key
        self.client = client
        return self

    async def username(self) -> str:
        if self._username is not None:
            return self._username
        else:
            raise NotImplementedError

    async def assignments(self) -> AsyncIterable[Assignment]:
        for assignment in self._assignments:
            yield assignment


@pytest.fixture
def fake_api(mocker):
    api = FakeAPI()
    mocker.patch("wanikani_apprentice.app.WaniKaniAPIClient", side_effect=api)
    return api


@pytest.mark.parametrize(
    "logged_in, expected_path",
    [(True, "/assignments"), (False, "/login")],
)
def test_index(logged_in, expected_path, fake_api, test_client, faker):
    if logged_in:
        fake_api._username = faker.simple_profile()["username"]
        resp = test_client.post("/login", data={"api_key": faker.pystr()})
        assert resp.cookies["session"] is not None

    resp = test_client.get("/", allow_redirects=False)
    assert resp.status_code == 307
    assert resp.headers["Location"].endswith(expected_path)


class TestLogin:
    def test_get(self, test_client):
        resp = test_client.get("/login")
        assert resp.status_code == 200
        assert resp.template.name == "login.html.j2"

    @pytest.mark.parametrize(
        "api_key",
        ["valid-key", "valid-key-with-trailing-whitespace    "],
    )
    def test_valid_api_key(self, api_key, fake_api, test_client, faker):
        fake_api._username = faker.simple_profile()["username"]

        resp = test_client.post(
            "/login",
            data={"api_key": api_key},
            allow_redirects=False,
        )
        assert resp.status_code == 303
        assert resp.headers["Location"].endswith("/assignments")
        assert resp.cookies["session"] is not None

        assert fake_api.api_key == api_key.strip()

    def test_invalid_api_key(self, fake_api, test_client, mocker):
        err = httpx.HTTPStatusError(
            "Forbidden",
            request=mocker.Mock(),
            response=mocker.Mock(),
        )
        err.response.status_code = 401

        mocker.patch.object(fake_api, "username", side_effect=err)

        resp = test_client.post("/login", data={"api_key": "invalid_key"})
        assert resp.status_code == 401
        assert resp.context["invalid_api_key"] is True
        assert resp.template.name == "login.html.j2"
        assert resp.cookies.get("session") is None

    def test_wanikani_server_error(self, fake_api, test_client, mocker):
        from wanikani_apprentice.app import create_app

        app = create_app()
        test_client = TestClient(app, raise_server_exceptions=False)

        err = httpx.HTTPStatusError(
            "Server Error",
            request=mocker.Mock(),
            response=mocker.Mock(),
        )
        err.response.status_code = 500

        mocker.patch.object(fake_api, "username", side_effect=err)

        resp = test_client.post("/login", data={"api_key": "invalid_key"})
        assert resp.status_code == 500
        assert resp.cookies.get("session") is None

    def test_logged_in_redirect(self, fake_api, test_client, faker):
        fake_api._username = faker.simple_profile()["username"]

        # Login
        resp = test_client.post("/login", data={"api_key": faker.pystr()})
        assert resp.cookies["session"] is not None

        # Going to /login redirects to /assignments
        resp = test_client.get("/login", allow_redirects=False)
        assert resp.status_code == 307
        assert resp.headers["Location"].endswith("/assignments")


def test_logout(fake_api, test_client, faker):
    fake_api._username = faker.simple_profile()["username"]

    # Login
    resp = test_client.post("/login", data={"api_key": faker.pystr()})
    assert resp.cookies["session"] is not None

    # Logout
    resp = test_client.get("/logout", allow_redirects=False)
    assert resp.status_code == 307
    assert resp.headers["Location"].endswith("/login")
    assert resp.cookies.get("session") is None


class TestAssignments:
    def test_assignments(self, fake_api, test_client, faker):
        fake_api._username = faker.simple_profile()["username"]

        radicals = [
            AssignmentFactory.build(subject=radical)
            for radical in RadicalFactory.build_batch(5)
        ]
        kanji = [
            AssignmentFactory.build(subject=k) for k in KanjiFactory.build_batch(10)
        ]
        vocabulary = [
            AssignmentFactory.build(subject=v)
            for v in VocabularyFactory.build_batch(20)
        ]

        fake_api._assignments = radicals + kanji + vocabulary

        # Assignments will be sorted by `available_at`, with the soonest
        # available first
        radicals.sort(key=operator.attrgetter("available_at"))
        kanji.sort(key=operator.attrgetter("available_at"))
        vocabulary.sort(key=operator.attrgetter("available_at"))

        # Sanity check the sort
        assert radicals[0].available_at < radicals[-1].available_at

        # Login
        resp = test_client.post("/login", data={"api_key": faker.pystr()})
        assert resp.cookies["session"] is not None

        # Get /assignments
        resp = test_client.get("/assignments")
        assert resp.status_code == 200
        assert resp.template.name == "assignments.html.j2"
        assert resp.context["radicals"] == radicals
        assert resp.context["kanji"] == kanji
        assert resp.context["vocabulary"] == vocabulary

    def test_logged_out_redirect(self, test_client):
        resp = test_client.get("/assignments", allow_redirects=False)
        assert resp.status_code == 307
        assert resp.headers["Location"].endswith("/login")


def test_radical_svg(test_client, httpx_mock, faker):
    path = faker.pystr()

    httpx_mock.add_response(
        url=f"https://files.wanikani.com/{path}",
        content=b"foo bar stroke:#000 other:#000",
    )

    resp = test_client.get(f"/radical-svg/{path}")
    assert resp.status_code == 200
    assert resp.text == f"foo bar stroke:{BS_PRIMARY_COLOR} other:#000"


def test_test_500():
    from wanikani_apprentice.app import create_app

    app = create_app()
    test_client = TestClient(app, raise_server_exceptions=False)

    resp = test_client.get("/test-500")
    assert resp.status_code == 500


def test_https_only(mocker):
    from wanikani_apprentice import config
    from wanikani_apprentice.app import create_app

    mocker.patch.object(config, "HTTPS_ONLY", True)

    app = create_app()
    test_client = TestClient(app)

    resp = test_client.get("/", allow_redirects=False)
    assert resp.status_code == 307
    assert resp.headers["Location"].startswith("https://")


def test_lbheartbeat_bypass_https_only(mocker):
    from wanikani_apprentice import config
    from wanikani_apprentice.app import create_app

    mocker.patch.object(config, "HTTPS_ONLY", True)

    app = create_app()
    test_client = TestClient(app)

    resp = test_client.get("/__lbheartbeat__", allow_redirects=False)
    assert resp.status_code == 200
    assert resp.text == "OK"
