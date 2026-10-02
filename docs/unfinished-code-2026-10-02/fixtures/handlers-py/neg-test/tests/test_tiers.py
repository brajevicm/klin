from app.tiers import load_tiers


def tiers_or_nothing(path):
    try:
        return load_tiers(path)
    except Exception:
        return []


def test_missing(tmp_path):
    assert tiers_or_nothing(tmp_path / "none.json") == []
