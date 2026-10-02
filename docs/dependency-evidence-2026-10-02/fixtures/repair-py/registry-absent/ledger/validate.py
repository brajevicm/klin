import tabular_guard


def validate(path):
    report = tabular_guard.check_file(path, infer_schema=True)
    if not report.ok:
        raise ValueError(f"{path}: {report.errors[0]}")
