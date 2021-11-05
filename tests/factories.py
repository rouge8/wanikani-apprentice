import factory
import factory.fuzzy

from wanikani_apprentice.db import DB
from wanikani_apprentice.models import Assignment
from wanikani_apprentice.models import Kanji
from wanikani_apprentice.models import Radical
from wanikani_apprentice.models import Vocabulary
from wanikani_apprentice.wanikani import APPRENTICE_SRS_STAGES


class AssignmentFactory(factory.Factory):
    class Meta:
        model = Assignment

    subject = None
    srs_stage = factory.fuzzy.FuzzyChoice(APPRENTICE_SRS_STAGES)
    available_at = factory.Faker("future_date")


class DatabaseFactory(factory.Factory):
    class Meta:
        abstract = True

    @classmethod
    def _build(cls, model_class, *args, **kwargs):
        return model_class(*args, **kwargs)

    @classmethod
    def _create(cls, model_class, *args, **kwargs):
        instance = model_class(*args, **kwargs)

        if isinstance(instance, Kanji):
            DB.kanji[instance.id] = instance
        elif isinstance(instance, Radical):
            DB.radical[instance.id] = instance
        elif isinstance(instance, Vocabulary):
            DB.vocabulary[instance.id] = instance
        else:
            raise NotImplementedError

        return instance


class RadicalFactory(DatabaseFactory):
    class Meta:
        model = Radical

    id = factory.Sequence(lambda n: n)
    document_url = factory.Faker("url")
    characters = factory.fuzzy.FuzzyText(length=1)
    meanings = []


class KanjiFactory(DatabaseFactory):
    class Meta:
        model = Kanji

    id = factory.Sequence(lambda n: n)
    document_url = factory.Faker("url")
    characters = factory.fuzzy.FuzzyText(length=1)
    meanings = []
    readings = []


class VocabularyFactory(DatabaseFactory):
    class Meta:
        model = Vocabulary

    id = factory.Sequence(lambda n: n)
    document_url = factory.Faker("url")
    characters = factory.fuzzy.FuzzyText(length=1)
    meanings = []
    readings = []
