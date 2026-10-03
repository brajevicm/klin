#!/usr/bin/env python3
"""Edit matrix for #425. Each case is a base tree and an edited tree. The probe commits the
base on `main`, writes the edit on `work`, and runs `klin gate --gate <gate> --json` from each
binary named on the command line, once whole and once with `--changed`. The last binary must
be the prototype: the identity columns are its keys for every site of the base and the edited
tree.

usage: probe.py NAME=BINARY [NAME=BINARY ...] > results.md
"""

import json
import subprocess
import sys
import tempfile
from pathlib import Path

COMPLEXITY = '{ "complexity": { "in": "src", "cc": 0, "lines": 1000 } }'

RS_BRANCH = "    if {v} > 0 {{ 1 }} else {{ 0 }}\n"


def rs_fn(name, arg="x", ret="i32", body=None, cc=2):
    body = body or "".join(
        f"    if {arg} > {k} {{ return {k}; }}\n" for k in range(cc - 1)
    ) + f"    {arg} * 2\n"
    return f"fn {name}({arg}: i32) -> {ret} {{\n{body}}}\n"


def rs_run(owner, cc, trait=None):
    branches = "".join(f"        if self.0 > {k} {{ return {k}; }}\n" for k in range(cc - 1))
    head = f"impl {trait} for {owner}" if trait else f"impl {owner}"
    return f"{head} {{\n    fn run(&self) -> u32 {{\n{branches}        7\n    }}\n}}\n"


def py_init(owner, cc):
    branches = "".join(f"        if a > {k}:\n            self.v = {k}\n" for k in range(cc - 1))
    return f"class {owner}:\n    def __init__(self, a):\n{branches}        self.w = a\n"


def ts_method(owner, cc, name="run"):
    branches = "".join(f"    if (this.v > {k}) return {k};\n" for k in range(cc - 1))
    return f"class {owner} {{\n  v = 0;\n  {name}(): number {{\n{branches}    return 7;\n  }}\n}}\n"


CASES = []


def case(name, base, after, desired, gate="complexity", config=COMPLEXITY):
    CASES.append(dict(name=name, base=base, after=after, desired=desired, gate=gate, config=config))


# 1 comment and blank lines above
case(
    "comment-above",
    {"src/a.rs": rs_fn("a")},
    {"src/a.rs": "// note\n\n" + rs_fn("a")},
    "held",
)
# 2 unrelated sibling inserted, and siblings reordered
case(
    "sibling-insert",
    {"src/a.rs": rs_fn("a")},
    {"src/a.rs": rs_fn("b") + rs_fn("a")},
    "a held, b new",
)
case(
    "sibling-reorder",
    {"src/a.rs": rs_fn("a") + rs_fn("b", cc=3)},
    {"src/a.rs": rs_fn("b", cc=3) + rs_fn("a")},
    "held",
)
# 3 body edit, same declaration, same values
case(
    "body-edit",
    {"src/a.rs": rs_fn("a")},
    {"src/a.rs": rs_fn("a").replace("x * 2", "x * 3")},
    "held",
)
# 4 signature edit
case(
    "signature-only",
    {"src/a.rs": rs_fn("a")},
    {"src/a.rs": rs_fn("a", ret="i64")},
    "held",
)
case(
    "signature-and-body",
    {"src/a.rs": rs_fn("a")},
    {"src/a.rs": rs_fn("a", ret="i64").replace("x * 2", "x * 3")},
    "held",
)
case(
    "visibility-added",
    {"src/a.rs": rs_fn("a")},
    {"src/a.rs": "pub " + rs_fn("a").replace("x * 2", "x * 3")},
    "held",
)
# 5 parameter rename (the body uses the parameter)
case(
    "parameter-rename",
    {"src/a.rs": rs_fn("a")},
    {"src/a.rs": rs_fn("a", arg="y")},
    "held",
)
case(
    "parameter-rename-ts",
    {"src/a.ts": "export function a(x: number): number {\n  if (x > 0) return 1;\n  return x;\n}\n"},
    {"src/a.ts": "export function a(y: number): number {\n  if (y > 0) return 1;\n  return y;\n}\n"},
    "held",
)
case(
    "parameter-rename-tsx",
    {"src/a.tsx": "export const Row = ({ x }: { x: number }) => {\n  if (x) return <b>{x}</b>;\n  return <i>{x}</i>;\n};\n"},
    {"src/a.tsx": "export const Row = ({ y }: { y: number }) => {\n  if (y) return <b>{y}</b>;\n  return <i>{y}</i>;\n};\n"},
    "held",
)
# 6 declaration rename
case(
    "rename-same-body",
    {"src/a.rs": rs_fn("a")},
    {"src/a.rs": rs_fn("b")},
    "held (body-hash proof)",
)
case(
    "rename-and-body",
    {"src/a.rs": rs_fn("a")},
    {"src/a.rs": rs_fn("b").replace("x * 2", "x * 3")},
    "new",
)
# 7 file move
case(
    "file-move",
    {"src/a.rs": rs_fn("a")},
    {"src/a.rs": None, "src/b.rs": rs_fn("a")},
    "held",
)
case(
    "file-move-and-signature",
    {"src/a.rs": rs_fn("a")},
    {"src/a.rs": None, "src/b.rs": rs_fn("a", arg="y")},
    "new: the key holds the file, and the body changed",
)
# 8 nested named functions
case(
    "nested-signature",
    {"src/a.rs": "fn outer() -> i32 {\n" + "".join("    " + l + "\n" for l in rs_fn("inner").splitlines()) + "    inner(1)\n}\n"},
    {"src/a.rs": "fn outer() -> i32 {\n" + "".join("    " + l + "\n" for l in rs_fn("inner", arg="y").splitlines()) + "    inner(1)\n}\n"},
    "held",
)
case(
    "nested-same-name-two-parents",
    {"src/a.rs": "".join(
        f"fn {o}() -> i32 {{\n" + "".join("    " + l + "\n" for l in rs_fn("inner", cc=c).splitlines()) + "    inner(1)\n}\n"
        for o, c in (("p", 4), ("q", 2)))},
    {"src/a.rs": "".join(
        f"fn {o}() -> i32 {{\n" + "".join("    " + l + "\n" for l in rs_fn("inner", cc=c).splitlines()) + "    inner(1)\n}\n"
        for o, c in (("p", 2), ("q", 4)))},
    "q > inner worsened 2 -> 4",
)
# 9 same method name on two owners: one improves, the other worsens past it
case(
    "two-owners-swap-rust",
    {"src/a.rs": "struct A(u32);\nstruct B(u32);\n" + rs_run("A", 5) + rs_run("B", 3)},
    {"src/a.rs": "struct A(u32);\nstruct B(u32);\n" + rs_run("A", 3) + rs_run("B", 5)},
    "B::run worsened 3 -> 5",
)
case(
    "two-owners-misnamed-rust",
    {"src/a.rs": "struct A(u32);\nstruct B(u32);\n" + rs_run("A", 5) + rs_run("B", 3)},
    {"src/a.rs": "struct A(u32);\nstruct B(u32);\n" + rs_run("A", 4) + rs_run("B", 5)},
    "B::run worsened 3 -> 5, A held",
)
case(
    "two-traits-swap-rust",
    {"src/a.rs": "struct A(u32);\n" + rs_run("A", 5, "P") + rs_run("A", 3, "Q")},
    {"src/a.rs": "struct A(u32);\n" + rs_run("A", 3, "P") + rs_run("A", 5, "Q")},
    "A as Q::run worsened 3 -> 5",
)
case(
    "two-owners-swap-python",
    {"src/a.py": py_init("A", 5) + py_init("B", 3)},
    {"src/a.py": py_init("A", 3) + py_init("B", 5)},
    "B.__init__ worsened 3 -> 5",
)
case(
    "two-owners-swap-ts",
    {"src/a.ts": ts_method("A", 5) + ts_method("B", 3)},
    {"src/a.ts": ts_method("A", 3) + ts_method("B", 5)},
    "B.run worsened 3 -> 5",
)
case(
    "owner-rename-same-body",
    {"src/a.rs": "struct A(u32);\n" + rs_run("A", 3)},
    {"src/a.rs": "struct B(u32);\n" + rs_run("B", 3)},
    "held (body-hash proof)",
)
case(
    "owner-rename-and-body",
    {"src/a.rs": "struct A(u32);\n" + rs_run("A", 3)},
    {"src/a.rs": "struct B(u32);\n" + rs_run("B", 3).replace("7", "8")},
    "today's outcome: neither key persists, so the text pairs them",
)
case(
    "method-moved-to-other-owner-and-edited",
    {"src/a.rs": "struct A(u32);\nstruct B(u32);\n" + rs_run("A", 3) + "impl B {}\n"},
    {"src/a.rs": "struct A(u32);\nstruct B(u32);\nimpl A {}\n" + rs_run("B", 3).replace("7", "8")},
    "today's outcome: neither key persists, so the text pairs them",
)
case(
    "twin-crosses-ceiling",
    {"src/a.rs": "struct A(u32);\nstruct B(u32);\n" + rs_run("A", 5) + rs_run("B", 2)},
    {"src/a.rs": "struct A(u32);\nstruct B(u32);\n" + rs_run("A", 1) + rs_run("B", 5)},
    "B::run new (it crossed the ceiling)",
    config='{ "complexity": { "in": "src", "cc": 2, "lines": 1000 } }',
)
# 10 static versus instance member
case(
    "static-and-instance-ts",
    {"src/a.ts": "class A {\n  static make(x: number): number {\n    if (x) return 1;\n    return 0;\n  }\n  make(x: number): number {\n    if (x) return 1;\n    return 0;\n  }\n}\n"},
    {"src/a.ts": "class A {\n  static make(y: number): number {\n    if (y) return 1;\n    return 0;\n  }\n  make(y: number): number {\n    if (y) return 1;\n    return 0;\n  }\n}\n"},
    "both held",
)
# 11 getter and setter
case(
    "getter-setter-ts",
    {"src/a.ts": "class A {\n  v = 0;\n  get value(): number {\n    if (this.v) return 1;\n    return 0;\n  }\n  set value(x: number) {\n    if (x) this.v = x;\n  }\n}\n"},
    {"src/a.ts": "class A {\n  v = 0;\n  get value(): number {\n    if (this.v) return 1;\n    return 0;\n  }\n  set value(y: number) {\n    if (y) this.v = y;\n  }\n}\n"},
    "both held",
)
case(
    "property-setter-python",
    {"src/a.py": "class A:\n    @property\n    def v(self):\n        if self.w:\n            return 1\n        return 0\n\n    @v.setter\n    def v(self, x):\n        if x:\n            self.w = x\n"},
    {"src/a.py": "class A:\n    @property\n    def v(self):\n        if self.w:\n            return 1\n        return 0\n\n    @v.setter\n    def v(self, y):\n        if y:\n            self.w = y\n"},
    "both held (ambiguous: today's outcome)",
)
# 12 duplicate candidate identities
case(
    "duplicate-key-signature",
    {"src/a.rs": "struct F<T>(T);\nimpl F<u8> {\n    fn new(x: i32) -> i32 {\n        if x > 0 { 1 } else { 0 }\n    }\n}\nimpl F<u16> {\n    fn new(x: i32) -> i32 {\n        if x > 1 { 1 } else { 0 }\n    }\n}\n"},
    {"src/a.rs": "struct F<T>(T);\nimpl F<u8> {\n    fn new(y: i32) -> i32 {\n        if y > 0 { 1 } else { 0 }\n    }\n}\nimpl F<u16> {\n    fn new(x: i32) -> i32 {\n        if x > 1 { 1 } else { 0 }\n    }\n}\n"},
    "today's outcome (ambiguous)",
)
# 13 anonymous and computed
case(
    "anonymous-callback-ts",
    {"src/a.ts": "export const ys = [1].map((x) => {\n  if (x) return 1;\n  return 0;\n});\n"},
    {"src/a.ts": "export const ys = [1].map((y) => {\n  if (y) return 1;\n  return 0;\n});\n"},
    "today's outcome (ambiguous)",
)
case(
    "computed-member-ts",
    {"src/a.ts": "const k = 'go';\nexport const o = {\n  [k](x: number) {\n    if (x) return 1;\n    return 0;\n  },\n};\n"},
    {"src/a.ts": "const k = 'go';\nexport const o = {\n  [k](y: number) {\n    if (y) return 1;\n    return 0;\n  },\n};\n"},
    "today's outcome (ambiguous)",
)
case(
    "bound-arrow-ts",
    {"src/a.ts": "export const f = (x: number) => {\n  if (x) return 1;\n  return 0;\n};\n"},
    {"src/a.ts": "export const f = (y: number) => {\n  if (y) return 1;\n  return 0;\n};\n"},
    "held",
)
case(
    "nested-under-callback-ts",
    {"src/a.ts": "export const ys = [1].map(() => {\n  function g(x: number) {\n    if (x) return 1;\n    return 0;\n  }\n  return g(1);\n});\n"},
    {"src/a.ts": "export const ys = [1].map(() => {\n  function g(y: number) {\n    if (y) return 1;\n    return 0;\n  }\n  return g(1);\n});\n"},
    "today's outcome (ambiguous ancestry)",
)
# 14 delete and add a replacement
case(
    "delete-add-replacement",
    {"src/a.rs": rs_fn("a")},
    {"src/a.rs": rs_fn("b", cc=3)},
    "b new",
)
case(
    "same-name-new-body",
    {"src/a.rs": rs_fn("a")},
    {"src/a.rs": "fn a(x: i32) -> i32 {\n    match x { 0 => 1, _ => 2 }\n}\n"},
    "held (same site, values decide)",
)
# 15 one existing occurrence plus one new duplicate
case(
    "added-duplicate-occurrence",
    {"src/a.rs": "struct F<T>(T);\nimpl F<u8> {\n    fn new(x: i32) -> i32 {\n        if x > 0 { 1 } else { 0 }\n    }\n}\n"},
    {"src/a.rs": "struct F<T>(T);\nimpl F<u8> {\n    fn new(x: i32) -> i32 {\n        if x > 0 { 1 } else { 0 }\n    }\n}\nimpl F<u16> {\n    fn new(x: i32) -> i32 {\n        if x > 0 { 1 } else { 0 }\n    }\n}\n"},
    "one held, one new",
)
case(
    "added-duplicate-occurrence-python",
    {"src/a.py": "def h(a):\n    if a:\n        return 1\n    return 0\n"},
    {"src/a.py": "def h(a):\n    if a:\n        return 1\n    return 0\n\n\ndef h(a):\n    if a:\n        return 1\n    return 0\n"},
    "one held, one new",
)
# accepted entries
ACCEPTED_A = ('{ "accepted": [{"gate": "complexity", "file": "src/a.rs", "text": "fn a(x: i32) -> i32 {", '
              '"cc": 2, "lines": 4}], "complexity": { "in": "src", "cc": 0, "lines": 1000 } }')
case(
    "accepted-param-rename",
    {},
    {"src/a.rs": rs_fn("a", arg="y")},
    "new, entry stale (a person edits the entry)",
    config=ACCEPTED_A,
)
case(
    "accepted-and-base-param-rename",
    {"src/a.rs": rs_fn("a")},
    {"src/a.rs": rs_fn("a", arg="y")},
    "held, entry stale under --strict",
    config=ACCEPTED_A,
)
case(
    "accepted-and-base-unchanged",
    {"src/a.rs": rs_fn("a")},
    {"src/a.rs": "// note\n" + rs_fn("a")},
    "held by the accepted entry",
    config=ACCEPTED_A,
)

# dead-symbols, current matcher only: the prototype gives this family no identity
DEAD = '{ "complexity": false }'
case(
    "dead-param-rename",
    {"src/lib.rs": "pub fn used() {}\nfn unused(x: i32) -> i32 {\n    x\n}\n"},
    {"src/lib.rs": "pub fn used() {}\nfn unused(y: i32) -> i32 {\n    y\n}\n"},
    "held",
    gate="dead-symbols",
    config=DEAD,
)
case(
    "dead-signature-only",
    {"src/lib.rs": "pub fn used() {}\nfn unused(x: i32) -> i32 {\n    x\n}\n"},
    {"src/lib.rs": "pub fn used() {}\nfn unused(x: i32) -> i64 {\n    x as i64\n}\n"},
    "held",
    gate="dead-symbols",
    config=DEAD,
)
case(
    "dead-comment-above",
    {"src/lib.rs": "pub fn used() {}\nfn unused(x: i32) -> i32 {\n    x\n}\n"},
    {"src/lib.rs": "pub fn used() {}\n// note\nfn unused(x: i32) -> i32 {\n    x\n}\n"},
    "held",
    gate="dead-symbols",
    config=DEAD,
)


def git(root, *args):
    subprocess.run(
        ["git", "-c", "user.name=probe", "-c", "user.email=probe@example.com",
         "-c", "commit.gpgsign=false", *args],
        cwd=root, check=True, capture_output=True,
    )


def write(root, files):
    for path, text in files.items():
        target = root / path
        if text is None:
            git(root, "rm", "-q", path)
            continue
        target.parent.mkdir(parents=True, exist_ok=True)
        target.write_text(text)


def tree(spec):
    root = Path(tempfile.mkdtemp(prefix="identity-"))
    git(root, "init", "-q", "-b", "main")
    (root / "klin.json").write_text(spec["config"])
    write(root, spec["base"])
    git(root, "add", "-A")
    git(root, "commit", "-q", "-m", "base")
    git(root, "checkout", "-q", "-b", "work")
    write(root, spec["after"])
    git(root, "add", "-A")
    return root


def outcome(binary, root, gate, changed):
    subprocess.run(["rm", "-rf", str(root / ".git" / "klin")], check=True)
    args = [binary, "gate", "--gate", gate, "--json", "--strict"]
    if changed:
        args.remove("--strict")
        args.append("--changed")
    run = subprocess.run(args, cwd=root, capture_output=True, text=True)
    data = json.loads(run.stdout)
    report = data["gates"][0]
    parts = []
    for finding in data["findings"]:
        values = finding["values"]
        metric = values.get("cc", values.get("dead"))
        matched = finding.get("matched") or {}
        was = (matched.get("values") or {}).get("cc") if isinstance(matched, dict) else None
        at = f"{finding['file']}:{finding['line']}"
        parts.append(f"{finding['outcome']} {at} {metric}" + (f" (was {was})" if was is not None else ""))
    stale = sum(1 for note in data["notes"] if "match" in note.get("text", "") or note.get("outcome") == "unmatched")
    held = report.get("held", 0)
    accepted = report.get("accepted", 0)
    head = f"{report['status']}, held {held}" + (f", accepted {accepted}" if accepted else "")
    if stale:
        head += f", {stale} note"
    return head + ("; " + "; ".join(parts) if parts else "")


def keys(binary, spec, files, named):
    """Every site's identity in one tree, as the prototype derives it: the tree is measured
    against an empty base, so every site is a finding and the JSON lists it."""
    if spec["gate"] != "complexity":
        return "n/a"
    root = Path(tempfile.mkdtemp(prefix="identity-keys-"))
    git(root, "init", "-q", "-b", "main")
    git(root, "commit", "-q", "--allow-empty", "-m", "empty")
    git(root, "checkout", "-q", "-b", "work")
    (root / "klin.json").write_text(COMPLEXITY)
    write(root, {path: text for path, text in files.items() if text is not None})
    git(root, "add", "-A")
    run = subprocess.run([binary, "gate", "--gate", "complexity", "--json"],
                         cwd=root, capture_output=True, text=True)
    subprocess.run(["rm", "-rf", str(root)], check=True)
    sites = []
    for finding in json.loads(run.stdout)["findings"]:
        identity = finding.get("values", {}).get("identity")
        if identity is None:
            continue
        shown = identity.split(":", 1)[1] if ":" in identity else "?" + identity.split("?", 1)[1]
        sites.append((finding["file"], finding["line"], shown.replace("|", "\\|")))
    if not sites:
        return "none"
    many = named or len({file for file, _, _ in sites}) > 1
    return "; ".join(f"{file + ':' if many else ''}{line} `{shown}`" for file, line, shown in sorted(sites))


def main():
    binaries = [arg.split("=", 1) for arg in sys.argv[1:]]
    identities = binaries[-1][1]
    columns = [f"{name} {mode}" for name, _ in binaries for mode in ("whole", "changed")]
    print("| case | gate | desired | identity before | identity after | " + " | ".join(columns) + " |")
    print("|" + "---|" * (5 + len(columns)))
    for spec in CASES:
        root = tree(spec)
        cells = [outcome(binary, root, spec["gate"], changed)
                 for _, binary in binaries for changed in (False, True)]
        edited = {**spec["base"], **spec["after"]}
        named = set(spec["base"]) != {path for path, text in edited.items() if text is not None}
        before = keys(identities, spec, spec["base"], named)
        after = keys(identities, spec, edited, named)
        print(f"| {spec['name']} | {spec['gate']} | {spec['desired']} | {before} | {after} | "
              + " | ".join(cells) + " |")
        subprocess.run(["rm", "-rf", str(root)], check=True)


if __name__ == "__main__":
    main()
