from app.report import to_json
from app.rows import load_rows


def main(path):
    return to_json(load_rows(path))
