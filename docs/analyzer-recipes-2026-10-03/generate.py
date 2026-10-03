#!/usr/bin/env python3
"""Write fixtures/recipes-ts and fixtures/recipes-py from the tables below.

Each route is the base tree with one or more edits. An edit replaces one
exact text of a base file, or writes a new file. `routes.tsv` names each
route's family and class.
"""

import pathlib
import shutil
import sys

HERE = pathlib.Path(__file__).resolve().parent

TS_BASE = {
    ".gitignore": ".klin-recipes/\nnode_modules/\n",
    "package.json": '{ "name": "shop", "private": true, "type": "module" }\n',
    "tsconfig.json": """{
  "compilerOptions": {
    "strict": true,
    "target": "ES2022",
    "module": "ESNext",
    "moduleResolution": "Bundler",
    "lib": ["ES2022", "DOM"],
    "noEmit": true
  },
  "include": ["src", "scripts", "tests"]
}
""",
    "src/env.d.ts": """declare module "node:child_process" {
  export function exec(command: string, done?: (error: unknown, out: string) => void): void;
  export function execFile(file: string, args: string[], done?: (error: unknown, out: string) => void): void;
}

declare const process: {
  env: Record<string, string | undefined>;
  stdout: { write(text: string): boolean };
  exit(code: number): never;
};
""",
    "src/db.ts": """export interface Db {
  query(sql: string, params?: unknown[]): Promise<Row[]>;
}

export interface Row {
  [column: string]: unknown;
}

export class NotFound extends Error {}
""",
    "src/log.ts": """export interface Logger {
  debug(message: string, context?: object): void;
  error(message: string, context?: object): void;
}

export const logger: Logger = {
  debug: () => undefined,
  error: () => undefined,
};
""",
    "src/config.ts": """export interface Config {
  apiUrl: string;
  apiKey: string;
  database: { host: string; user: string; password: string };
}

function required(name: string): string {
  const value = process.env[name];
  if (value === undefined) {
    throw new Error(`${name} is not set`);
  }
  return value;
}

export function load(): Config {
  return {
    apiUrl: required("API_URL"),
    apiKey: required("API_KEY"),
    database: {
      host: required("DB_HOST"),
      user: required("DB_USER"),
      password: required("DB_PASSWORD"),
    },
  };
}
""",
    "src/users.ts": """import type { Db, Row } from "./db";

export interface User {
  id: number;
  name: string;
  email: string;
}

function toUser(row: Row): User {
  return { id: Number(row.id), name: String(row.name), email: String(row.email) };
}

export async function findByName(db: Db, name: string): Promise<User[]> {
  const rows = await db.query("SELECT id, name, email FROM users WHERE name = $1", [name]);
  return rows.map(toUser);
}

export async function findById(db: Db, id: number): Promise<User | undefined> {
  const rows = await db.query("SELECT id, name, email FROM users WHERE id = $1", [id]);
  const first = rows[0];
  return first ? toUser(first) : undefined;
}
""",
    "src/orders.ts": """import type { Db } from "./db";
import { NotFound } from "./db";
import { logger } from "./log";

export interface Item {
  sku: string;
  cents: number;
  quantity: number;
}

export interface Order {
  id: number;
  items: Item[];
}

export function total(order: Order): number {
  let sum = 0;
  for (const item of order.items) {
    sum += item.cents * item.quantity;
  }
  return sum;
}

export async function load(db: Db, id: number): Promise<Order> {
  const rows = await db.query("SELECT items FROM orders WHERE id = $1", [id]);
  const first = rows[0];
  if (!first) {
    throw new NotFound(`order ${id}`);
  }
  return { id, items: JSON.parse(String(first.items)) as Item[] };
}

export async function totalOf(db: Db, id: number): Promise<number> {
  try {
    return total(await load(db, id));
  } catch (error) {
    logger.error("order total failed", { id, error });
    throw error;
  }
}
""",
    "src/images.ts": """import { execFile } from "node:child_process";

export function thumbnail(file: string, done: (error: unknown) => void): void {
  execFile("convert", [file, "-resize", "128x128", `${file}.thumb.png`], (error) => done(error));
}
""",
    "src/price.ts": """export function format(cents: number): string {
  const whole = Math.trunc(cents / 100);
  const rest = Math.abs(cents % 100).toString().padStart(2, "0");
  return `${whole}.${rest}`;
}

export function parse(text: string): number {
  const [whole, rest = "0"] = text.split(".");
  return Number(whole) * 100 + Number(rest.padEnd(2, "0").slice(0, 2));
}
""",
    "scripts/report.ts": """import { format } from "../src/price";

const totals = [1250, 399, 10000];
for (const cents of totals) {
  console.log(format(cents));
}
""",
    "tests/price.test.ts": """import { format, parse } from "../src/price";

declare function test(name: string, body: () => void): void;
declare function expect(value: unknown): { toBe(other: unknown): void };

test("format", () => {
  expect(format(1250)).toBe("12.50");
});

test("parse", () => {
  expect(parse("3.99")).toBe(399);
});
""",
}

KEY = "q8Zt3vX9mL2pR7wN4kJ6hB1cF5dS0aYe"
PASSWORD = "Winter2026!Prod"

FIND_BY_NAME = 'const rows = await db.query("SELECT id, name, email FROM users WHERE name = $1", [name]);'
THUMB = 'execFile("convert", [file, "-resize", "128x128", `${file}.thumb.png`], (error) => done(error));'
TOTAL_OF_CATCH = """  } catch (error) {
    logger.error("order total failed", { id, error });
    throw error;
  }"""
LOAD_KEY = '    apiKey: required("API_KEY"),'
LOAD_PASSWORD = '      password: required("DB_PASSWORD"),'
TOTAL_LOOP = """  for (const item of order.items) {
    sum += item.cents * item.quantity;
  }"""

TS_ROUTES = {
    # injection
    "inj-plant-sql": ("injection", "plant", {"src/users.ts": [(FIND_BY_NAME, "const rows = await db.query(`SELECT id, name, email FROM users WHERE name = '${name}'`);")]}),
    "inj-plant-exec": ("injection", "plant", {"src/images.ts": [
        ('import { execFile } from "node:child_process";', 'import { exec } from "node:child_process";'),
        (THUMB, "exec(`convert ${file} -resize 128x128 ${file}.thumb.png`, (error) => done(error));")]}),
    "inj-plant-eval": ("injection", "plant", {"src/price.ts": [
        ("export function parse(text: string): number {\n  const [whole, rest = \"0\"] = text.split(\".\");\n  return Number(whole) * 100 + Number(rest.padEnd(2, \"0\").slice(0, 2));\n}",
         "export function parse(text: string): number {\n  return Math.round(Number(eval(text)) * 100);\n}")]}),
    "inj-neg-param": ("injection", "negative", {"src/users.ts": [(FIND_BY_NAME, "const rows = await db.query(`SELECT id, name, email FROM users WHERE name = $1`, [name]);")]}),
    "inj-neg-const": ("injection", "negative", {"src/users.ts": [
        ('import type { Db, Row } from "./db";', 'import type { Db, Row } from "./db";\n\nconst COLUMNS = "id, name, email";'),
        (FIND_BY_NAME, "const rows = await db.query(`SELECT ${COLUMNS} FROM users WHERE name = $1`, [name]);")]}),
    "inj-reword-concat": ("injection", "rewording", {"src/users.ts": [(FIND_BY_NAME, "const rows = await db.query(\"SELECT id, name, email FROM users WHERE name = '\" + name + \"'\");")]}),
    "inj-reword-variable": ("injection", "rewording", {"src/users.ts": [(FIND_BY_NAME, "const sql = `SELECT id, name, email FROM users WHERE name = '${name}'`;\n  const rows = await db.query(sql);")]}),
    "inj-reword-join": ("injection", "rewording", {"src/users.ts": [(FIND_BY_NAME, "const sql = [\"SELECT id, name, email FROM users WHERE name = '\", name, \"'\"].join(\"\");\n  const rows = await db.query(sql);")]}),
    "inj-reword-function": ("injection", "rewording", {"src/price.ts": [
        ("export function parse(text: string): number {\n  const [whole, rest = \"0\"] = text.split(\".\");\n  return Number(whole) * 100 + Number(rest.padEnd(2, \"0\").slice(0, 2));\n}",
         "export function parse(text: string): number {\n  const evaluate = new Function(`return (${text});`) as () => unknown;\n  return Math.round(Number(evaluate()) * 100);\n}")]}),
    "inj-suppress-eslint": ("injection", "suppression", {"src/price.ts": [
        ("export function parse(text: string): number {\n  const [whole, rest = \"0\"] = text.split(\".\");\n  return Number(whole) * 100 + Number(rest.padEnd(2, \"0\").slice(0, 2));\n}",
         "export function parse(text: string): number {\n  // eslint-disable-next-line no-eval\n  return Math.round(Number(eval(text)) * 100);\n}")]}),
    "inj-suppress-nosemgrep": ("injection", "suppression", {"src/users.ts": [(FIND_BY_NAME, "// nosemgrep\n  const rows = await db.query(`SELECT id, name, email FROM users WHERE name = '${name}'`);")]}),
    "inj-legit": ("injection", "legitimate", {"src/users.ts": [
        ("export async function findById(", "export async function findByEmail(db: Db, email: string): Promise<User[]> {\n  const rows = await db.query(\"SELECT id, name, email FROM users WHERE email = $1\", [email]);\n  return rows.map(toUser);\n}\n\nexport async function findById(")]}),
    # secrets
    "sec-plant-key": ("secrets", "plant", {"src/config.ts": [(LOAD_KEY, f'    apiKey: "{KEY}",')]}),
    "sec-plant-password": ("secrets", "plant", {"src/config.ts": [(LOAD_PASSWORD, f'      password: "{PASSWORD}",')]}),
    "sec-plant-constant": ("secrets", "plant", {"src/config.ts": [
        ("export interface Config {", f'const API_KEY = "{KEY}";\n\nexport interface Config {{'),
        (LOAD_KEY, "    apiKey: API_KEY,")]}),
    "sec-neg-name": ("secrets", "negative", {"src/config.ts": [
        ("export interface Config {", 'export const PASSWORD_FIELD = "password";\nexport const TOKEN_HEADER = "x-api-token";\n\nexport interface Config {')]}),
    "sec-neg-test": ("secrets", "negative", {"tests/config.test.ts": [(None, 'declare function test(name: string, body: () => void): void;\n\nconst fixture = { user: "test", password: "test-password" };\n\ntest("fixture", () => {\n  void fixture;\n});\n')]}),
    "sec-reword-split": ("secrets", "rewording", {"src/config.ts": [(LOAD_KEY, f'    apiKey: ["{KEY[:12]}", "{KEY[12:]}"].join(""),')]}),
    "sec-reword-base64": ("secrets", "rewording", {"src/config.ts": [(LOAD_KEY, '    apiKey: atob("cThadDN2WDltTDJwUjd3TjRrSjZoQjFjRjVkUzBhWWU="),')]}),
    "sec-reword-rename": ("secrets", "rewording", {"src/config.ts": [
        ("export interface Config {", f'const DEFAULT_A = "{KEY}";\n\nexport interface Config {{'),
        (LOAD_KEY, "    apiKey: DEFAULT_A,")]}),
    "sec-suppress-gitleaks": ("secrets", "suppression", {"src/config.ts": [(LOAD_KEY, f'    apiKey: "{KEY}", // gitleaks:allow')]}),
    "sec-legit": ("secrets", "legitimate", {"src/config.ts": [(LOAD_KEY, '    apiKey: required("SHOP_API_KEY"),')]}),
    # swallowed errors
    "err-plant-empty": ("swallowed", "plant", {"src/orders.ts": [(TOTAL_OF_CATCH, "  } catch (error) {}\n  return 0;")]}),
    "err-plant-promise": ("swallowed", "plant", {"src/orders.ts": [
        ("export async function totalOf(", "export function warm(db: Db, id: number): void {\n  load(db, id).catch(() => {});\n}\n\nexport async function totalOf(")]}),
    "err-plant-default": ("swallowed", "plant", {"src/orders.ts": [(TOTAL_OF_CATCH, "  } catch {\n    return 0;\n  }")]}),
    "err-neg-rethrow": ("swallowed", "negative", {"src/orders.ts": [(TOTAL_OF_CATCH, "  } catch (error) {\n    if (error instanceof NotFound) {\n      return 0;\n    }\n    throw error;\n  }")]}),
    "err-reword-comment": ("swallowed", "rewording", {"src/orders.ts": [(TOTAL_OF_CATCH, "  } catch (error) {\n    // ignore\n  }\n  return 0;")]}),
    "err-reword-void": ("swallowed", "rewording", {"src/orders.ts": [(TOTAL_OF_CATCH, "  } catch (error) {\n    void error;\n  }\n  return 0;")]}),
    "err-suppress-eslint": ("swallowed", "suppression", {"src/orders.ts": [(TOTAL_OF_CATCH, "    // eslint-disable-next-line no-empty\n  } catch (error) {}\n  return 0;")]}),
    "err-legit": ("swallowed", "legitimate", {"src/orders.ts": [(TOTAL_OF_CATCH, "  } catch (error) {\n    logger.error(\"order total failed\", { id, error });\n    throw new Error(`total of order ${id} failed`, { cause: error });\n  }")]}),
    # dead code
    "dead-plant-import": ("dead", "plant", {"src/orders.ts": [('import { logger } from "./log";', 'import { logger } from "./log";\nimport { format } from "./price";')]}),
    "dead-plant-variable": ("dead", "plant", {"src/orders.ts": [("  let sum = 0;\n", "  let sum = 0;\n  const legacy = order.items.length * 2;\n")]}),
    "dead-plant-unreachable": ("dead", "plant", {"src/orders.ts": [("  return sum;\n}", "  return sum;\n  sum = 0;\n}")]}),
    "dead-plant-commented": ("dead", "plant", {"src/orders.ts": [(TOTAL_LOOP, TOTAL_LOOP + "\n  // for (const item of order.items) {\n  //   sum += item.cents;\n  // }")]}),
    "dead-neg-callback": ("dead", "negative", {"src/orders.ts": [("export async function load(", "export function skus(order: Order): string[] {\n  return order.items.map((_item, index) => `${order.id}-${index}`);\n}\n\nexport async function load(")]}),
    "dead-reword-export": ("dead", "rewording", {"src/orders.ts": [("export function total(", "export const legacyFactor = 2;\n\nexport function total(")]}),
    "dead-reword-void": ("dead", "rewording", {"src/orders.ts": [("  let sum = 0;\n", "  let sum = 0;\n  const legacy = order.items.length * 2;\n  void legacy;\n")]}),
    "dead-legit": ("dead", "legitimate", {"src/orders.ts": [("export async function load(", "export function count(order: Order): number {\n  return order.items.reduce((sum, item) => sum + item.quantity, 0);\n}\n\nexport async function load(")]}),
    # debug output
    "debug-plant-log": ("debug", "plant", {"src/orders.ts": [("  let sum = 0;\n", "  console.log(\"total\", order);\n  let sum = 0;\n")]}),
    "debug-plant-debugger": ("debug", "plant", {"src/orders.ts": [("  let sum = 0;\n", "  debugger;\n  let sum = 0;\n")]}),
    "debug-neg-script": ("debug", "negative", {"scripts/report.ts": [("  console.log(format(cents));", "  console.log(format(cents));\n  console.log(\"-\");")]}),
    "debug-neg-test": ("debug", "negative", {"tests/price.test.ts": [('  expect(format(1250)).toBe("12.50");', '  console.log(format(1250));\n  expect(format(1250)).toBe("12.50");')]}),
    "debug-reword-alias": ("debug", "rewording", {"src/orders.ts": [("  let sum = 0;\n", "  const { log } = console;\n  log(\"total\", order);\n  let sum = 0;\n")]}),
    "debug-reword-stdout": ("debug", "rewording", {"src/orders.ts": [("  let sum = 0;\n", "  process.stdout.write(`total ${JSON.stringify(order)}\\n`);\n  let sum = 0;\n")]}),
    "debug-suppress-eslint": ("debug", "suppression", {"src/orders.ts": [("  let sum = 0;\n", "  // eslint-disable-next-line no-console\n  console.log(\"total\", order);\n  let sum = 0;\n")]}),
    "debug-suppress-inline-config": ("debug", "suppression", {"src/orders.ts": [('import type { Db } from "./db";', '/* eslint no-console: "off" */\nimport type { Db } from "./db";'), ("  let sum = 0;\n", "  console.log(\"total\", order);\n  let sum = 0;\n")]}),
    "debug-legit": ("debug", "legitimate", {"src/orders.ts": [("  let sum = 0;\n", "  logger.debug(\"total\", { id: order.id });\n  let sum = 0;\n")]}),
    # unnecessary conditions
    "cond-plant-guard": ("condition", "plant", {"src/orders.ts": [("  let sum = 0;\n", "  if (!order.items) {\n    return 0;\n  }\n  let sum = 0;\n")]}),
    "cond-plant-optional": ("condition", "plant", {"src/orders.ts": [("    sum += item.cents * item.quantity;", "    sum += (item?.cents ?? 0) * item.quantity;")]}),
    "cond-plant-undefined": ("condition", "plant", {"src/price.ts": [("  const whole = Math.trunc(cents / 100);", "  if (cents === undefined || cents === null) {\n    return \"0.00\";\n  }\n  const whole = Math.trunc(cents / 100);")]}),
    "cond-neg-index": ("condition", "negative", {"src/orders.ts": [("export async function load(", "export function firstSku(order: Order): string {\n  const first = order.items[0];\n  return first ? first.sku : \"\";\n}\n\nexport async function load(")]}),
    "cond-neg-env": ("condition", "negative", {"src/config.ts": [("  if (value === undefined) {", "  if (value === undefined || value === \"\") {")]}),
    "cond-reword-widen": ("condition", "rewording", {"src/orders.ts": [("  items: Item[];\n}", "  items?: Item[];\n}"), ("  let sum = 0;\n  for (const item of order.items) {", "  if (!order.items) {\n    return 0;\n  }\n  let sum = 0;\n  for (const item of order.items) {")]}),
    "cond-reword-cast": ("condition", "rewording", {"src/orders.ts": [("  let sum = 0;\n", "  if (!(order.items as Item[] | undefined)) {\n    return 0;\n  }\n  let sum = 0;\n")]}),
    "cond-reword-any": ("condition", "rewording", {"src/orders.ts": [("  let sum = 0;\n", "  if (!(order as any).items) {\n    return 0;\n  }\n  let sum = 0;\n")]}),
    "cond-legit": ("condition", "legitimate", {"src/orders.ts": [("export async function load(", "export function isEmpty(order: Order): boolean {\n  return order.items.length === 0;\n}\n\nexport async function load(")]}),
}

PY_BASE = {
    ".gitignore": ".klin-recipes/\n__pycache__/\n",
    "pyproject.toml": '[project]\nname = "shop"\nversion = "0.1.0"\nrequires-python = ">=3.11"\n',
    "shop/__init__.py": "",
    "shop/config.py": '''import os
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
        api_key=required("API_KEY"),
        database=Database(
            host=required("DB_HOST"),
            user=required("DB_USER"),
            password=required("DB_PASSWORD"),
        ),
    )
''',
    "shop/users.py": '''import sqlite3
from dataclasses import dataclass


@dataclass(frozen=True)
class User:
    id: int
    name: str
    email: str


def find_by_name(db: sqlite3.Connection, name: str) -> list[User]:
    rows = db.execute("SELECT id, name, email FROM users WHERE name = ?", (name,)).fetchall()
    return [User(*row) for row in rows]


def find_by_id(db: sqlite3.Connection, user_id: int) -> User | None:
    row = db.execute("SELECT id, name, email FROM users WHERE id = ?", (user_id,)).fetchone()
    return User(*row) if row else None
''',
    "shop/orders.py": '''import json
import logging
import sqlite3
from dataclasses import dataclass

log = logging.getLogger(__name__)


class NotFound(Exception):
    pass


@dataclass(frozen=True)
class Item:
    sku: str
    cents: int
    quantity: int


@dataclass(frozen=True)
class Order:
    id: int
    items: list[Item]


def total(order: Order) -> int:
    result = 0
    for item in order.items:
        result += item.cents * item.quantity
    return result


def load(db: sqlite3.Connection, order_id: int) -> Order:
    row = db.execute("SELECT items FROM orders WHERE id = ?", (order_id,)).fetchone()
    if row is None:
        raise NotFound(f"order {order_id}")
    return Order(order_id, [Item(**item) for item in json.loads(row[0])])


def total_of(db: sqlite3.Connection, order_id: int) -> int:
    try:
        return total(load(db, order_id))
    except (NotFound, json.JSONDecodeError):
        log.exception("order total failed", extra={"order": order_id})
        raise
''',
    "shop/images.py": '''import subprocess


def thumbnail(path: str) -> None:
    subprocess.run(["convert", path, "-resize", "128x128", f"{path}.thumb.png"], check=True)
''',
    "shop/price.py": '''def format_cents(cents: int) -> str:
    whole, rest = divmod(abs(cents), 100)
    sign = "-" if cents < 0 else ""
    return f"{sign}{whole}.{rest:02d}"


def parse(text: str) -> int:
    whole, _, rest = text.partition(".")
    return int(whole) * 100 + int((rest or "0").ljust(2, "0")[:2])
''',
    "shop/cli.py": '''import sys

from shop.price import format_cents


def main(argv: list[str]) -> int:
    for arg in argv:
        print(format_cents(int(arg)))
    return 0


if __name__ == "__main__":
    sys.exit(main(sys.argv[1:]))
''',
    "tests/__init__.py": "",
    "tests/test_price.py": '''from shop.price import format_cents, parse


def test_format() -> None:
    assert format_cents(1250) == "12.50"


def test_parse() -> None:
    assert parse("3.99") == 399
''',
}

PY_FIND = '    rows = db.execute("SELECT id, name, email FROM users WHERE name = ?", (name,)).fetchall()'
PY_THUMB = '    subprocess.run(["convert", path, "-resize", "128x128", f"{path}.thumb.png"], check=True)'
PY_PARSE = '''    whole, _, rest = text.partition(".")
    return int(whole) * 100 + int((rest or "0").ljust(2, "0")[:2])'''
PY_KEY = '        api_key=required("API_KEY"),'
PY_PASSWORD = '            password=required("DB_PASSWORD"),'
PY_CATCH = '''    except (NotFound, json.JSONDecodeError):
        log.exception("order total failed", extra={"order": order_id})
        raise'''
PY_LOOP_START = "    result = 0\n"
PY_LOOP = '''    for item in order.items:
        result += item.cents * item.quantity'''

PY_ROUTES = {
    "inj-plant-sql": ("injection", "plant", {"shop/users.py": [(PY_FIND, '    rows = db.execute(f"SELECT id, name, email FROM users WHERE name = \'{name}\'").fetchall()')]}),
    "inj-plant-exec": ("injection", "plant", {"shop/images.py": [(PY_THUMB, '    subprocess.run(f"convert {path} -resize 128x128 {path}.thumb.png", shell=True, check=True)')]}),
    "inj-plant-eval": ("injection", "plant", {"shop/price.py": [(PY_PARSE, "    return round(float(eval(text)) * 100)")]}),
    "inj-neg-param": ("injection", "negative", {"shop/users.py": [(PY_FIND, '    query = "SELECT id, name, email FROM users WHERE name = ?"\n    rows = db.execute(query, (name,)).fetchall()')]}),
    "inj-neg-const": ("injection", "negative", {"shop/users.py": [
        ("from dataclasses import dataclass\n", 'from dataclasses import dataclass\n\nCOLUMNS = "id, name, email"\n'),
        (PY_FIND, '    rows = db.execute(f"SELECT {COLUMNS} FROM users WHERE name = ?", (name,)).fetchall()')]}),
    "inj-reword-concat": ("injection", "rewording", {"shop/users.py": [(PY_FIND, '    rows = db.execute("SELECT id, name, email FROM users WHERE name = \'" + name + "\'").fetchall()')]}),
    "inj-reword-variable": ("injection", "rewording", {"shop/users.py": [(PY_FIND, '    sql = f"SELECT id, name, email FROM users WHERE name = \'{name}\'"\n    rows = db.execute(sql).fetchall()')]}),
    "inj-reword-join": ("injection", "rewording", {"shop/users.py": [(PY_FIND, '    sql = "".join(["SELECT id, name, email FROM users WHERE name = \'", name, "\'"])\n    rows = db.execute(sql).fetchall()')]}),
    "inj-reword-function": ("injection", "rewording", {"shop/price.py": [("def parse(", "import builtins\n\n\ndef parse("), (PY_PARSE, '    evaluate = getattr(builtins, "ev" + "al")\n    return round(float(evaluate(text)) * 100)')]}),
    "inj-suppress-noqa": ("injection", "suppression", {"shop/price.py": [(PY_PARSE, "    return round(float(eval(text)) * 100)  # noqa: S307")]}),
    "inj-suppress-file": ("injection", "suppression", {"shop/price.py": [("def format_cents(", "# ruff: noqa: S307\n\n\ndef format_cents("), (PY_PARSE, "    return round(float(eval(text)) * 100)")]}),
    "inj-suppress-nosemgrep": ("injection", "suppression", {"shop/users.py": [(PY_FIND, '    rows = db.execute(f"SELECT id, name, email FROM users WHERE name = \'{name}\'").fetchall()  # nosemgrep  # noqa: S608')]}),
    "inj-legit": ("injection", "legitimate", {"shop/users.py": [("def find_by_id(", 'def find_by_email(db: sqlite3.Connection, email: str) -> list[User]:\n    rows = db.execute("SELECT id, name, email FROM users WHERE email = ?", (email,)).fetchall()\n    return [User(*row) for row in rows]\n\n\ndef find_by_id(')]}),
    "sec-plant-key": ("secrets", "plant", {"shop/config.py": [(PY_KEY, f'        api_key="{KEY}",')]}),
    "sec-plant-password": ("secrets", "plant", {"shop/config.py": [(PY_PASSWORD, f'            password="{PASSWORD}",')]}),
    "sec-plant-constant": ("secrets", "plant", {"shop/config.py": [("from dataclasses import dataclass\n", f'from dataclasses import dataclass\n\nAPI_KEY = "{KEY}"\n'), (PY_KEY, "        api_key=API_KEY,")]}),
    "sec-neg-name": ("secrets", "negative", {"shop/config.py": [("from dataclasses import dataclass\n", 'from dataclasses import dataclass\n\nPASSWORD_FIELD = "password"\nTOKEN_HEADER = "x-api-token"\n')]}),
    "sec-neg-test": ("secrets", "negative", {"tests/test_config.py": [(None, 'FIXTURE = {"user": "test", "password": "test-password"}\n\n\ndef test_fixture() -> None:\n    assert FIXTURE["user"] == "test"\n')]}),
    "sec-reword-split": ("secrets", "rewording", {"shop/config.py": [(PY_KEY, f'        api_key="".join(["{KEY[:12]}", "{KEY[12:]}"]),')]}),
    "sec-reword-base64": ("secrets", "rewording", {"shop/config.py": [("import os\n", "import base64\nimport os\n"), (PY_KEY, '        api_key=base64.b64decode("cThadDN2WDltTDJwUjd3TjRrSjZoQjFjRjVkUzBhWWU=").decode(),')]}),
    "sec-reword-rename": ("secrets", "rewording", {"shop/config.py": [("from dataclasses import dataclass\n", f'from dataclasses import dataclass\n\nDEFAULT_A = "{KEY}"\n'), (PY_KEY, "        api_key=DEFAULT_A,")]}),
    "sec-suppress-gitleaks": ("secrets", "suppression", {"shop/config.py": [(PY_KEY, f'        api_key="{KEY}",  # gitleaks:allow')]}),
    "sec-legit": ("secrets", "legitimate", {"shop/config.py": [(PY_KEY, '        api_key=required("SHOP_API_KEY"),')]}),
    "err-plant-pass": ("swallowed", "plant", {"shop/orders.py": [(PY_CATCH, "    except Exception:\n        pass\n    return 0")]}),
    "err-plant-default": ("swallowed", "plant", {"shop/orders.py": [(PY_CATCH, "    except Exception:\n        return 0")]}),
    "err-plant-continue": ("swallowed", "plant", {"shop/orders.py": [("def total_of(", 'def totals(db: sqlite3.Connection, order_ids: list[int]) -> list[int]:\n    results = []\n    for order_id in order_ids:\n        try:\n            results.append(total(load(db, order_id)))\n        except Exception:\n            continue\n    return results\n\n\ndef total_of(')]}),
    "err-neg-rethrow": ("swallowed", "negative", {"shop/orders.py": [(PY_CATCH, "    except Exception as error:\n        raise RuntimeError(f\"total of order {order_id} failed\") from error")]}),
    "err-neg-specific": ("swallowed", "negative", {"shop/orders.py": [(PY_CATCH, "    except NotFound:\n        return 0")]}),
    "err-reword-narrow": ("swallowed", "rewording", {"shop/orders.py": [(PY_CATCH, "    except (NotFound, ValueError, KeyError, TypeError):\n        pass\n    return 0")]}),
    "err-reword-suppress": ("swallowed", "rewording", {"shop/orders.py": [("import json\n", "import contextlib\nimport json\n"), ("def total_of(db: sqlite3.Connection, order_id: int) -> int:\n    try:\n        return total(load(db, order_id))\n" + PY_CATCH, "def total_of(db: sqlite3.Connection, order_id: int) -> int:\n    with contextlib.suppress(Exception):\n        return total(load(db, order_id))\n    return 0")]}),
    "err-reword-name": ("swallowed", "rewording", {"shop/orders.py": [(PY_CATCH, "    except Exception as error:\n        _ = error\n    return 0")]}),
    "err-suppress-noqa": ("swallowed", "suppression", {"shop/orders.py": [(PY_CATCH, "    except Exception:  # noqa: BLE001, S110\n        pass\n    return 0")]}),
    "err-legit": ("swallowed", "legitimate", {"shop/orders.py": [(PY_CATCH, "    except NotFound:\n        log.info(\"order %s not found\", order_id)\n        raise")]}),
    "dead-plant-import": ("dead", "plant", {"shop/orders.py": [("import sqlite3\n", "import sqlite3\nimport time\n")]}),
    "dead-plant-variable": ("dead", "plant", {"shop/orders.py": [(PY_LOOP_START, "    result = 0\n    legacy = len(order.items) * 2\n")]}),
    "dead-plant-unreachable": ("dead", "plant", {"shop/orders.py": [("    return result\n", "    return result\n    result = 0\n")]}),
    "dead-plant-commented": ("dead", "plant", {"shop/orders.py": [(PY_LOOP, PY_LOOP + "\n    # for item in order.items:\n    #     result += item.cents")]}),
    "dead-neg-reexport": ("dead", "negative", {"shop/__init__.py": [(None, "from shop.orders import Order, total\n\n__all__ = [\"Order\", \"total\"]\n")]}),
    "dead-reword-underscore": ("dead", "rewording", {"shop/orders.py": [(PY_LOOP_START, "    result = 0\n    _legacy = len(order.items) * 2\n")]}),
    "dead-reword-import-as": ("dead", "rewording", {"shop/orders.py": [("import sqlite3\n", "import sqlite3\nimport time as time\n")]}),
    "dead-legit": ("dead", "legitimate", {"shop/orders.py": [("def load(", "def count(order: Order) -> int:\n    return sum(item.quantity for item in order.items)\n\n\ndef load(")]}),
    "debug-plant-print": ("debug", "plant", {"shop/orders.py": [(PY_LOOP_START, '    print("total", order)\n    result = 0\n')]}),
    "debug-plant-pprint": ("debug", "plant", {"shop/orders.py": [("import json\n", "import json\nfrom pprint import pprint\n"), (PY_LOOP_START, "    pprint(order)\n    result = 0\n")]}),
    "debug-plant-breakpoint": ("debug", "plant", {"shop/orders.py": [(PY_LOOP_START, "    breakpoint()\n    result = 0\n")]}),
    "debug-neg-cli": ("debug", "negative", {"shop/cli.py": [("        print(format_cents(int(arg)))", "        print(format_cents(int(arg)))\n    print(\"-\")")]}),
    "debug-neg-test": ("debug", "negative", {"tests/test_price.py": [('    assert format_cents(1250) == "12.50"', '    print(format_cents(1250))\n    assert format_cents(1250) == "12.50"')]}),
    "debug-reword-stdout": ("debug", "rewording", {"shop/orders.py": [("import json\n", "import json\nimport sys\n"), (PY_LOOP_START, '    sys.stdout.write(f"total {order}\\n")\n    result = 0\n')]}),
    "debug-reword-alias": ("debug", "rewording", {"shop/orders.py": [("log = logging.getLogger(__name__)\n", "log = logging.getLogger(__name__)\nshow = print\n"), (PY_LOOP_START, '    show("total", order)\n    result = 0\n')]}),
    "debug-suppress-noqa": ("debug", "suppression", {"shop/orders.py": [(PY_LOOP_START, '    print("total", order)  # noqa: T201\n    result = 0\n')]}),
    "debug-legit": ("debug", "legitimate", {"shop/orders.py": [(PY_LOOP_START, '    log.debug("total of order %s", order.id)\n    result = 0\n')]}),
}


def apply(base: dict[str, str], edits: dict[str, list]) -> dict[str, str]:
    changed = {}
    for path, replacements in edits.items():
        text = base.get(path)
        for old, new in replacements:
            if old is None:
                text = new
                continue
            if text is None or text.count(old) != 1:
                sys.exit(f"{path}: the text to replace occurs {0 if text is None else text.count(old)} times: {old!r}")
            text = text.replace(old, new)
        changed[path] = text
    return changed


def write(root: pathlib.Path, files: dict[str, str]) -> None:
    for path, text in files.items():
        target = root / path
        target.parent.mkdir(parents=True, exist_ok=True)
        target.write_text(text)


def family(name: str, base: dict[str, str], routes: dict) -> None:
    root = HERE / "fixtures" / name
    shutil.rmtree(root, ignore_errors=True)
    write(root / "base", base)
    rows = []
    for route, (kind, klass, edits) in routes.items():
        write(root / route, apply(base, edits))
        rows.append(f"{route}\t{kind}\t{klass}\n")
    (root / "routes.tsv").write_text("".join(rows))


family("recipes-ts", TS_BASE, TS_ROUTES)
family("recipes-py", PY_BASE, PY_ROUTES)
