import importlib

csvshape = importlib.import_module("csvshape")


def validate(path):
    return csvshape.Schema.infer(path).validate(path)
