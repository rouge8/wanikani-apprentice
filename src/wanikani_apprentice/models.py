from datetime import datetime

import attr


@attr.frozen
class Radical:
    id: int
    document_url: str
    characters: str
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
