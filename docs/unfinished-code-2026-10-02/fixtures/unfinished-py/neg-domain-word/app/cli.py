from app.report import soft_404, to_csv, to_json
from app.rows import load_rows


def main(path, fmt):
    soft_404(path)
    rows = load_rows(path)
    return to_csv(rows) if fmt == "csv" else to_json(rows)
