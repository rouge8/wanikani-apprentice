from datetime import datetime
from datetime import timedelta

import attr
from babel.dates import format_timedelta


@attr.frozen
class Radical:
    id: int
    document_url: str
    characters: str | None
    character_svg_path: str | None
    meanings: list[str]


@attr.frozen
class Kanji:
    id: int
    document_url: str
    characters: str
    meanings: list[str]
    readings: list[str]


@attr.frozen
class Vocabulary:
    id: int
    document_url: str
    characters: str
    meanings: list[str]
    readings: list[str]


@attr.frozen
class Assignment:
    subject: Radical | Kanji | Vocabulary
    srs_stage: int
    available_at: datetime

    def display_time_remaining(self, now: datetime) -> str:
        delta = self.available_at - now

        if delta > timedelta(0):
            return format_timedelta(delta, add_direction=True)  # type: ignore
        else:
            return "now"
