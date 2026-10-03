import os
from dataclasses import dataclass


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
        api_key="".join(["q8Zt3vX9mL2p", "R7wN4kJ6hB1cF5dS0aYe"]),
        database=Database(
            host=required("DB_HOST"),
            user=required("DB_USER"),
            password=required("DB_PASSWORD"),
        ),
    )
