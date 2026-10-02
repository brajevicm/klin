from httpx import Client


def fetch_rates(url):
    with Client(timeout=10) as client:
        return client.get(url).json()
