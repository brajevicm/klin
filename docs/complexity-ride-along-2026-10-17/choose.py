"""Select the sample, the dry run and the reserve of #697.

A copy of the pilot's chooser (docs/phenotype-pilot-2026-10-02/choose.py):
the same candidates, agent rotation and GitHub eligibility checks, with the
salt 697:, the #697 exclusions and no human arm.

usage: uv run --with duckdb python choose.py AIDEV_DIR CLONES
"""

import hashlib
import json
import pathlib
import shutil
import subprocess
import sys
import time

import duckdb

HERE = pathlib.Path(__file__).resolve().parent
ROOT = HERE.parent.parent
SAMPLE = 200
DRY = 10
RESERVE = {"Rust": None, "TypeScript": 50}
AGENTS = ["Claude_Code", "OpenAI_Codex", "Cursor", "Copilot", "Devin", "Google_Jules"]
EXTENSIONS = {
    "Rust": (".rs",),
    "TypeScript": (".ts", ".tsx", ".mts", ".cts"),
}
MAX_KB = 150_000
MAX_FILES = 100


def gh(path, **fields):
    args = ["gh", "api", "-X", "GET", path]
    for key, value in fields.items():
        args += ["-f", f"{key}={value}"]
    done = retried(args)
    if done.returncode != 0:
        if "HTTP 404" in done.stderr:
            return None
        raise RuntimeError(f"gh api {path}: {done.stderr.strip()}")
    return json.loads(done.stdout)


def retried(args):
    for attempt in range(5):
        done = subprocess.run(args, capture_output=True, text=True)
        if done.returncode == 0 or "HTTP 404" in done.stderr:
            return done
        time.sleep(15 * (attempt + 1))
    return done


def commits_of(full_name, number):
    args = ["gh", "api", "--paginate", "--slurp", f"repos/{full_name}/pulls/{number}/commits?per_page=100"]
    done = retried(args)
    if done.returncode != 0:
        raise RuntimeError(f"gh api {args[-1]}: {done.stderr.strip()}")
    return [commit for page in json.loads(done.stdout) for commit in page]


def git(cwd, *args):
    return subprocess.run(["git", "-C", str(cwd), *args], capture_output=True, text=True)


def fetched(directory, ref):
    for attempt in range(3):
        if git(directory, "fetch", "--quiet", "origin", ref).returncode == 0:
            return True
        time.sleep(15 * (attempt + 1))
    return False


def bare(url):
    return url.lower().removeprefix("https://github.com/").removesuffix(".git")


def excluded():
    pilot = json.loads((ROOT / "docs/phenotype-pilot-2026-10-02/selection.json").read_text())
    names = {record["fullName"].lower() for arm in ("agent", "human") for record in pilot[arm]}
    alarms = json.loads((ROOT / "benchmark/evidence/false-alarms-2026-09-29/selection.json").read_text())
    names |= {record["fullName"].lower() for record in alarms["repositories"]}
    for path in (ROOT / "scripts/replay/samples").glob("*.json"):
        if not path.name.startswith("agents697"):
            names |= {bare(change["repo"]) for change in json.loads(path.read_text())["changes"]}
    return names


def candidates(aidev, skip):
    rows = duckdb.connect().sql(
        f"""
        select p.id, p.number, p.agent, p.created_at, r.language, r.full_name
        from '{aidev}/pull_request.parquet' p join '{aidev}/repository.parquet' r on p.repo_id = r.id
        where r.language in ('Rust', 'TypeScript') and not r.is_forked and p.closed_at is not null
        """
    ).fetchall()
    return [row for row in rows if row[5].lower() not in skip]


def order(rows, language):
    lists = {agent: [] for agent in AGENTS}
    for row in rows:
        if row[4] == language and row[2] in lists:
            lists[row[2]].append(row)
    for agent in AGENTS:
        lists[agent].sort(key=lambda row: hashlib.sha256(f"697:{row[0]}".encode()).hexdigest())
    while any(lists.values()):
        for agent in AGENTS:
            if lists[agent]:
                yield lists[agent].pop(0)


def clone(clones, full_name):
    directory = clones / full_name.replace("/", "__")
    if not directory.exists():
        for attempt in range(3):
            done = subprocess.run(
                ["git", "clone", "--quiet", "--no-tags", "--no-checkout", "--filter=blob:none", f"https://github.com/{full_name}.git", str(directory)],
                capture_output=True,
            )
            if done.returncode == 0:
                break
            shutil.rmtree(directory, ignore_errors=True)
            time.sleep(15 * (attempt + 1))
        else:
            return None
    return directory


def eligible(clones, repositories, language, full_name, number):
    """Return a change record, or the rule the pull request failed."""
    if full_name.lower() not in repositories:
        repositories[full_name.lower()] = gh(f"repos/{full_name}")
    repository = repositories[full_name.lower()]
    if repository is None:
        return "repository not readable"
    if repository["size"] > MAX_KB:
        return f"size {repository['size']} KB over {MAX_KB}"
    full_name = repository["full_name"]
    pull = gh(f"repos/{full_name}/pulls/{number}")
    if pull is None:
        return "pull request not readable"
    if pull["changed_files"] > MAX_FILES:
        return f"{pull['changed_files']} files over {MAX_FILES}"
    directory = clone(clones, full_name)
    if directory is None:
        return "clone failed"
    head = pull["head"]["sha"]
    base = pull["base"]["sha"]
    if not fetched(directory, f"pull/{number}/head"):
        return "head not fetchable"
    if git(directory, "rev-parse", "FETCH_HEAD").stdout.strip() != head:
        return "fetched head differs from the API head"
    if git(directory, "cat-file", "-e", f"{base}^{{commit}}").returncode != 0:
        git(directory, "fetch", "--quiet", "origin", base)
    merge_base = git(directory, "merge-base", base, head).stdout.strip()
    if not merge_base:
        return "no merge base"
    names = git(directory, "diff", "--name-only", merge_base, head).stdout.split()
    if not any(name.endswith(EXTENSIONS[language]) for name in names):
        return "no file of the language"
    if len(names) > MAX_FILES:
        return f"{len(names)} files over {MAX_FILES}"
    if git(directory, "cat-file", "-e", f"{head}:klin.json").returncode == 0:
        return "klin.json at the root"
    return {
        "language": language,
        "fullName": full_name,
        "number": number,
        "url": pull["html_url"],
        "createdAt": pull["created_at"],
        "merged": pull["merged_at"] is not None,
        "base": merge_base,
        "head": head,
        "files": len(names),
        "commits": len(commits_of(full_name, number)),
    }


def change(record):
    return {
        "repo": f"https://github.com/{record['fullName']}",
        "commit": record["head"],
        "base": record["base"],
        "pull": record["number"],
        "language": record["language"],
    }


def choose(clones, rows, language):
    """Walk the hash order once. Rust's dry run takes a second pull request of a sample repository."""
    lists = {"sample": [], "dry": [], "reserve": [], "skipped": []}
    repositories = {}
    taken = {}
    second = language == "Rust"
    for pr_id, number, agent, _, _, full_name in order(rows, language):
        if not second and len(lists["dry"]) == DRY and len(lists["reserve"]) == RESERVE[language]:
            break
        key = full_name.lower()
        if key in taken:
            if not (second and taken[key] == "sample" and len(lists["dry"]) < DRY):
                lists["skipped"].append({"id": pr_id, "fullName": full_name, "number": number, "rule": "repository already taken"})
                continue
        picked = eligible(clones, repositories, language, full_name, number)
        if isinstance(picked, str):
            lists["skipped"].append({"id": pr_id, "fullName": full_name, "number": number, "rule": picked})
            print(f"skip {language} {full_name}#{number}: {picked}", flush=True)
            continue
        if key in taken:
            into = "dry"
        elif len(lists["sample"]) < SAMPLE:
            into = "sample"
        elif not second and len(lists["dry"]) < DRY:
            into = "dry"
        else:
            into = "reserve"
        taken.setdefault(key, into)
        taken.setdefault(picked["fullName"].lower(), into)
        if into == "dry" and second:
            taken[key] = taken[picked["fullName"].lower()] = "dry"
        lists[into].append({"id": pr_id, "agent": agent, **picked})
        print(f"{into} {language} {agent} {picked['fullName']}#{number}", flush=True)
    return lists


def main(aidev, clones):
    clones = pathlib.Path(clones)
    clones.mkdir(parents=True, exist_ok=True)
    rows = candidates(aidev, excluded())
    selection = {}
    for language in EXTENSIONS:
        selection[language] = choose(clones, rows, language)
    (HERE / "selection.json").write_text(json.dumps(selection, indent=2, ensure_ascii=False) + "\n")
    for name, part in (("agents697", "sample"), ("agents697dry", "dry")):
        changes = [change(record) for language in EXTENSIONS for record in selection[language][part]]
        path = ROOT / "scripts/replay/samples" / f"{name}.json"
        path.write_text(json.dumps({"changes": changes}, indent=2) + "\n")


if __name__ == "__main__":
    main(*sys.argv[1:])
