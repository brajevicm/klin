import httpx


def fetch_rates(url):
    return httpx.get(url, timeout=10).json()
