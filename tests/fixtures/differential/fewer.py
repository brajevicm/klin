import os


def one():
    value = os.environ  # type: ignore
    return value


def two():
    return os.getcwd()


def three():
    return os.getcwd()
