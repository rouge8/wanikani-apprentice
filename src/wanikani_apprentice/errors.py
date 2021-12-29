import attrs


@attrs.frozen
class UnknownSubjectError(Exception):
    subject_id: int
