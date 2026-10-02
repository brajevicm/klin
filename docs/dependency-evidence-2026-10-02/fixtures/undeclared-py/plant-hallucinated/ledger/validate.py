import csvshape


def validate(path):
    return csvshape.Schema.infer(path).validate(path)
