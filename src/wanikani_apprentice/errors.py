import attr


@attr.frozen
class UnknownSubjectError(Exception):
    subject_id: int
