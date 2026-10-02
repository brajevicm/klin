try:
    import csvshape
except ImportError:
    csvshape = None


def validate(path):
    return csvshape.Schema.infer(path).validate(path)
