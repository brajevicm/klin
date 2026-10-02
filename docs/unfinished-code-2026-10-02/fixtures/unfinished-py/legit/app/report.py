import json


def to_json(rows):
    return json.dumps(rows)


def to_csv(rows):
    import csv
    import io

    out = io.StringIO()
    writer = csv.writer(out)
    writer.writerow(["name", "amount"])
    for row in rows:
        writer.writerow([row["name"], row["amount"]])
    return out.getvalue()
