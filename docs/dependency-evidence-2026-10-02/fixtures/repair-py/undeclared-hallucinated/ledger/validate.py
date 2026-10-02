import csvshape


def validate(path):
    schema = csvshape.Schema.infer(path)
    errors = schema.validate(path)
    if errors:
        raise ValueError(f"{path}: {errors[0]}")
