import os

# Every declaration below now sits three lines lower than it did.
# A site is keyed by the text of its line, so the shift alone must
# not unbaseline it. The bare except at the end is the one new site.


def one():
    value = os.environ  # type: ignore
    return value


def two():
    value = os.environ  # type: ignore
    return value


def three():
    return os.getcwd()  # noqa


def four():
    try:
        return os.getcwd()
    except:
        return None
