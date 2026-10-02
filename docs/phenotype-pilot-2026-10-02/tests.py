"""Print the assertion lines that a change removes from test files.

usage: python3 tests.py CLONE BASE HEAD
Prints one tab-separated row per site: family, file, base line, removed text.
"""

import re
import subprocess
import sys

TEST_SEGMENTS = {
    "test", "tests", "__tests__", "spec", "specs", "testing", "e2e", "__mocks__", "mocks",
    "testdata", "test_utils", "testutils",
}
EXTENSIONS = (".rs", ".ts", ".tsx", ".mts", ".cts", ".py", ".pyi")
TOKENS = {
    "rs": ("assert!", "assert_eq!", "assert_ne!", "assert_matches!", "debug_assert", ".expect_err(", "#[should_panic"),
    "ts": ("expect(", "assert(", "assert.", ".toBe", ".toEqual", ".toMatch", ".toThrow", ".rejects", ".resolves"),
    "py": ("assert ", "self.assert", "pytest.raises", "pytest.warns", ".assert_called", ".assert_awaited"),
}
HUNK = re.compile(r"^@@ -(\d+)(?:,\d+)? \+\d+(?:,\d+)? @@")


def git(clone, *args):
    return subprocess.run(["git", "-C", clone, *args], capture_output=True, text=True).stdout


def path_is_test(path):
    *dirs, name = path.split("/")
    return (
        any(directory in TEST_SEGMENTS for directory in dirs)
        or name.startswith("test_")
        or name.endswith(("_test.py", "_test.rs"))
        or name in ("conftest.py", "tests.rs")
        or any(part in name for part in (".test.", ".spec.", ".stories."))
    )


def kind(path):
    return "rs" if path.endswith(".rs") else "py" if path.endswith((".py", ".pyi")) else "ts"


def first_test_line(clone, base, path):
    """The first line of an inline Rust test module, or None."""
    for number, line in enumerate(git(clone, "show", f"{base}:{path}").splitlines(), 1):
        if "#[cfg(test)]" in line:
            return number
    return None


def asserts(line, tokens):
    return any(token in line for token in tokens)


def sites(clone, base, head):
    changed = git(clone, "diff", "--name-only", "--no-renames", "--diff-filter=M", base, head).split("\n")
    for path in filter(None, changed):
        if not path.endswith(EXTENSIONS):
            continue
        floor = 1
        if not path_is_test(path):
            if not path.endswith(".rs"):
                continue
            floor = first_test_line(clone, base, path)
            if floor is None:
                continue
        tokens = TOKENS[kind(path)]
        hunks = []
        for line in git(clone, "diff", "-U0", "--no-renames", base, head, "--", path).splitlines():
            found = HUNK.match(line)
            if found:
                hunks.append({"at": int(found.group(1)), "removed": [], "added": False})
            elif hunks and line.startswith("-") and not line.startswith("---"):
                hunk = hunks[-1]
                number = hunk["at"] + len(hunk.get("seen", []))
                hunk.setdefault("seen", []).append(line)
                if number >= floor and asserts(line[1:], tokens):
                    hunk["removed"].append((number, line[1:].strip()))
            elif hunks and line.startswith("+") and not line.startswith("+++") and asserts(line[1:], tokens):
                hunks[-1]["added"] = True
        for hunk in hunks:
            family = "assertion-rewritten" if hunk["added"] else "assertion-removed"
            for number, text in hunk["removed"]:
                yield family, path, number, text


if __name__ == "__main__":
    for row in sites(*sys.argv[1:4]):
        print("\t".join(str(part) for part in row))
