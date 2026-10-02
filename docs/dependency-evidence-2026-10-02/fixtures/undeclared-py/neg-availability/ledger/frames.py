import importlib.util


def is_pandas_available():
    return importlib.util.find_spec("pandas") is not None


if is_pandas_available():
    import pandas


def to_frame(rows):
    if not is_pandas_available():
        raise RuntimeError("install pandas to use to_frame")
    return pandas.DataFrame(rows)
