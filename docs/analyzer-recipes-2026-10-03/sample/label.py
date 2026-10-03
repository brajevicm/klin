#!/usr/bin/env python3
"""Write labels.tsv from failures.tsv and the drafted labels below.

The labeler is the agent that wrote the note. Every `dead` failure outside
the every-second sample of the registered rule stays unlabeled.
"""

import csv
import pathlib
import re

HERE = pathlib.Path(__file__).resolve().parent
FAMILIES = {r["rule"].removeprefix("recipes."): r["family"] for r in csv.DictReader(open(HERE.parent / "recipes/families.tsv"), delimiter="\t")}

APPROPRIATE = {
    ("lib/inngest/functions.ts", "290", "@typescript-eslint/no-empty-function"): "the release of an alert claim drops a failed update with no reason",
    ("src/cache/inmemory/entityStore.ts", "421", "@typescript-eslint/no-unnecessary-condition"): "storeObject is already checked by the && on the line above",
    ("src/cache/inmemory/readFromStore.ts", "541", "@typescript-eslint/no-unnecessary-condition"): "objectOrReference is a non-null parameter on this path",
    ("components/SearchCommand.tsx", "84", "@typescript-eslint/no-unnecessary-condition"): "displayStocks is always an array here",
    ("components/stocks/StockHeader.tsx", "53", "@typescript-eslint/no-unnecessary-condition"): "quote?.h repeats a chain that quote?.l already guards",
    ("integration-tests/type-tests/cacheOverride/invalid/index.ts", "2", "@typescript-eslint/no-unused-vars"): "an import the file never uses",
    ("lib/inngest/functions.ts", "3", "@typescript-eslint/no-unused-vars"): "an import the change left unused",
    ("lib/inngest/functions.ts", "4", "@typescript-eslint/no-unused-vars"): "an import the change left unused",
}

NOT_APPROPRIATE = [
    (r"no-console", r"__tests__/", "a test asserts on console.warn"),
    (r"no-console", r".", "operational server or error logging, not a debug print"),
    (r"no-empty-function", r".", "an intended no-op callback or default"),
    (r"^no-empty$|no-ignored-exceptions", r".", "a best-effort probe or cleanup with a fallback after it"),
    (r"no-unnecessary-condition", r".", "a runtime guard the types cannot see, or a flow-analysis gap"),
    (r"no-unused-vars", r"__tests__/|type-tests/|__benches__/", "a disposable, a type assertion or a test binding used for its effect"),
    (r"no-unused-vars", r".", "a parameter the signature requires or a binding of a handled catch"),
    (r"no-img-element", r".", "the project's own directive names a rule the recipe does not load"),
    (r"^S10[56]$", r"tests?/", "a test value, not a credential"),
    (r"^BLE001$", r".", "a documented best-effort fallback that logs"),
    (r"^T201$", r".", "a script's intended output"),
    (r"^F401$", r"__init__\.py$", "a re-export in a package __init__"),
    (r"^ERA001$", r".", "a comment that explains data, not code"),
    (r"hardcoded-secret", r".", "a map of names to config file paths"),
]

# The project's own configuration turns the rule off for the file (section 11).
PROJECT_OFF = {("huggingface/trl", "F401", r"__init__\.py$")}

TEST = re.compile(
    r"(^|/)(tests?|__tests__|spec|specs|testing|e2e|__mocks__|mocks|testdata|test_utils|testutils)/"
    r"|(^|/)test_[^/]*\.py$|_test\.py$|(^|/)conftest\.py$|\.(test|spec|stories)\.[^/]+$"
)


def label(row, sampled):
    lang, name, head, gate, file, line, rule, _ = row
    if rule == "@typescript-eslint/no-unused-vars" and not sampled:
        return "unlabeled", "outside the every-second sample"
    if (file, line, rule) in APPROPRIATE:
        return "appropriate", APPROPRIATE[(file, line, rule)]
    for rule_pattern, path_pattern, why in NOT_APPROPRIATE:
        if re.search(rule_pattern, rule) and re.search(path_pattern, file):
            return "not-appropriate", why
    raise SystemExit(f"no label for {row}")


rows = [line.rstrip("\n").split("\t") for line in open(HERE / "failures.tsv")]
dead = [i for i, row in enumerate(rows) if row[6] == "@typescript-eslint/no-unused-vars"]
sampled = set(dead[::2])
with open(HERE / "labels.tsv", "w") as out:
    out.write("language\trepository\thead\tgate\tfile\tline\trule\tfamily\tlabel\ttest\tproject-off\treason\n")
    for i, row in enumerate(rows):
        lang, name, head, gate, file, line, rule, _ = row
        verdict, reason = label(row, i in sampled)
        family = FAMILIES.get(rule, "config")
        off = any(name == n and rule == r and re.search(p, file) for n, r, p in PROJECT_OFF)
        out.write("\t".join([lang, name, head, gate, file, line, rule, family, verdict,
                             "test" if TEST.search(file) else "-", "project-off" if off else "-", reason]) + "\n")
