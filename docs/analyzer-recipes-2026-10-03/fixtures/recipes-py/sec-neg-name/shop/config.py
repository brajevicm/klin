import os
from dataclasses import dataclass

PASSWORD_FIELD = "password"
TOKEN_HEADER = "x-api-token"


@dataclass(frozen=True)
class Database:
    host: str
    user: str
    password: str


@dataclass(frozen=True)
class Config:
    api_url: str
    api_key: str
    database: Database


def required(name: str) -> str:
    value = os.environ.get(name)
    if value is None:
        raise RuntimeError(f"{name} is not set")
    return value


def load() -> Config:
    return Config(
        api_url=required("API_URL"),
        api_key=required("API_KEY"),
        database=Database(
            host=required("DB_HOST"),
            user=required("DB_USER"),
            password=required("DB_PASSWORD"),
        ),
    )
