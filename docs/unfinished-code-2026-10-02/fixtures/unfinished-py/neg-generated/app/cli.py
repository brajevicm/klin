from app.report import to_csv, to_json
from app.rows import load_rows


def main(path, fmt):
    rows = load_rows(path)
    return to_csv(rows) if fmt == "csv" else to_json(rows)
