FIXTURE = {"user": "test", "password": "test-password"}


def test_fixture() -> None:
    assert FIXTURE["user"] == "test"
