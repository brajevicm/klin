import sqlite3
from dataclasses import dataclass


@dataclass(frozen=True)
class User:
    id: int
    name: str
    email: str


def find_by_name(db: sqlite3.Connection, name: str) -> list[User]:
    sql = f"SELECT id, name, email FROM users WHERE name = '{name}'"
    rows = db.execute(sql).fetchall()
    return [User(*row) for row in rows]


def find_by_id(db: sqlite3.Connection, user_id: int) -> User | None:
    row = db.execute("SELECT id, name, email FROM users WHERE id = ?", (user_id,)).fetchone()
    return User(*row) if row else None
