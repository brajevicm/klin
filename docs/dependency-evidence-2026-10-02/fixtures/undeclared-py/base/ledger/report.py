import json

import requests
import yaml
from dateutil import parser


def load_rules(path):
    with open(path) as handle:
        return yaml.safe_load(handle)


def fetch_rates(url):
    return requests.get(url, timeout=10).json()


def parse_day(text):
    return parser.isoparse(text).date()


def to_json(rows):
    return json.dumps(rows)
