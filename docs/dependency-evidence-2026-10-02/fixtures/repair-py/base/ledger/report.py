import csv
import io

import requests
from dateutil import parser


def fetch_rates(url):
    return requests.get(url, timeout=10).json()


def parse_day(text):
    return parser.isoparse(text).date()


def to_csv(rows):
    out = io.StringIO()
    writer = csv.DictWriter(out, fieldnames=list(rows[0]) if rows else [])
    writer.writeheader()
    writer.writerows(rows)
    return out.getvalue()
