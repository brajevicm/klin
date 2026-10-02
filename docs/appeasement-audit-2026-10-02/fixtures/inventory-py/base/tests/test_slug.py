import pytest

from app.slug import slugify


def test_joins_words_with_a_dash():
    assert slugify("Hello World") == "hello-world"


def test_strips_accents():
    assert slugify("Crème Brûlée") == "creme-brulee"


def test_trims_dashes_at_both_ends():
    assert slugify("  Hi!  ") == "hi"
