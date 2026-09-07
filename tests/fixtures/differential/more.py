import os


def one():
    value = os.environ  # type: ignore
    return value


def two():
    value = os.environ  # type: ignore
    return value


def three():
    return os.getcwd()  # noqa


def four():
    value = os.environ  # type: ignore
    return value
