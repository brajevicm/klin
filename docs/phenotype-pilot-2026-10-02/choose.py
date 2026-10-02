"""Select the agent and human arms of the #357 pilot.

usage: uv run --with duckdb python choose.py AIDEV_DIR CLONES
"""

import datetime
import hashlib
import json
import pathlib
import re
import subprocess
import sys

import duckdb

HERE = pathlib.Path(__file__).resolve().parent
PER_LANGUAGE = 10
AGENTS = ["Claude_Code", "OpenAI_Codex", "Cursor", "Copilot", "Devin", "Google_Jules"]
EXTENSIONS = {
    "Rust": (".rs",),
    "TypeScript": (".ts", ".tsx", ".mts", ".cts"),
    "Python": (".py", ".pyi"),
}
MAX_KB = 150_000
MAX_FILES = 100
WINDOW_DAYS = 60
AGENT_BRANCHES = ("codex/", "cursor/", "copilot/", "devin/", "claude/", "jules/", "jules-")
AGENT_WORDS = re.compile(
    r"claude code|co-authored-by: claude|codex|cursor agent|cursoragent|devin|jules|copilot|generated with|\U0001F916",
    re.IGNORECASE,
)


def gh(path, **fields):
    args = ["gh", "api", "-X", "GET", path]
    for key, value in fields.items():
        args += ["-f", f"{key}={value}"]
    done = subprocess.run(args, capture_output=True, text=True)
    if done.returncode != 0:
        return None
    return json.loads(done.stdout)


def git(cwd, *args):
    return subprocess.run(["git", "-C", str(cwd), *args], capture_output=True, text=True)


def candidates(aidev):
    db = duckdb.connect()
    rows = db.sql(
        f"""
        select p.id, p.number, p.agent, p.created_at, r.language, r.full_name
        from '{aidev}/pull_request.parquet' p join '{aidev}/repository.parquet' r on p.repo_id = r.id
        where r.language in ('Rust', 'TypeScript', 'Python') and not r.is_forked and p.closed_at is not null
        """
    ).fetchall()
    known = {
        (name.lower(), number)
        for name, number in db.sql(
            f"select r.full_name, p.number from '{aidev}/pull_request.parquet' p join '{aidev}/repository.parquet' r on p.repo_id = r.id"
        ).fetchall()
    }
    return rows, known


def order(rows, language):
    lists = {agent: [] for agent in AGENTS}
    for row in rows:
        if row[4] == language and row[2] in lists:
            lists[row[2]].append(row)
    for agent in AGENTS:
        lists[agent].sort(key=lambda row: hashlib.sha256(f"357:{row[0]}".encode()).hexdigest())
    while any(lists.values()):
        for agent in AGENTS:
            if lists[agent]:
                yield lists[agent].pop(0)


def clone(clones, full_name):
    directory = clones / full_name.replace("/", "__")
    if not directory.exists():
        done = subprocess.run(
            ["git", "clone", "--quiet", "--no-tags", f"https://github.com/{full_name}.git", str(directory)],
            capture_output=True,
        )
        if done.returncode != 0:
            return None
    return directory


def eligible(clones, language, full_name, number):
    """Return a change record, or the rule the pull request failed."""
    repository = gh(f"repos/{full_name}")
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
    if git(directory, "fetch", "--quiet", "origin", f"pull/{number}/head").returncode != 0:
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
    commits = gh(f"repos/{full_name}/pulls/{number}/commits", per_page=100) or []
    authors = sorted({(c.get("author") or {}).get("login") or c["commit"]["author"]["name"] for c in commits})
    return {
        "language": language,
        "fullName": full_name,
        "number": number,
        "url": pull["html_url"],
        "createdAt": pull["created_at"],
        "merged": pull["merged_at"] is not None,
        "author": pull["user"]["login"],
        "headRef": pull["head"]["ref"],
        "base": merge_base,
        "head": head,
        "files": len(names),
        "commitAuthors": authors,
        "_commits": commits,
        "_pull": pull,
    }


def human(pull, commits):
    user = pull["user"]
    if user["type"] == "Bot" or user["login"].endswith("[bot]") or user["login"] == "Copilot":
        return "author is a bot or an agent"
    if pull["head"]["ref"].startswith(AGENT_BRANCHES):
        return "agent branch"
    texts = [pull.get("body") or ""] + [c["commit"]["message"] for c in commits]
    if any(AGENT_WORDS.search(text) for text in texts):
        return "agent word in the body or a commit"
    return None


def human_for(clones, known, agent):
    created = datetime.datetime.fromisoformat(agent["createdAt"].replace("Z", "+00:00"))
    low = (created - datetime.timedelta(days=WINDOW_DAYS)).date()
    high = (created + datetime.timedelta(days=WINDOW_DAYS)).date()
    items = []
    for page in range(1, 11):
        found = gh(
            "search/issues",
            q=f"repo:{agent['fullName']} is:pr is:closed created:{low}..{high}",
            per_page=100,
            page=page,
        )
        if not found or not found["items"]:
            break
        items += found["items"]
        if len(found["items"]) < 100:
            break

    def distance(item):
        at = datetime.datetime.fromisoformat(item["created_at"].replace("Z", "+00:00"))
        return (abs((at - created).total_seconds()), item["number"])

    skipped = []
    for item in sorted(items, key=distance):
        if item["number"] == agent["number"]:
            continue
        if (agent["fullName"].lower(), item["number"]) in known:
            skipped.append({"number": item["number"], "rule": "in the AIDev pull_request table"})
            continue
        picked = eligible(clones, agent["language"], agent["fullName"], item["number"])
        if isinstance(picked, str):
            skipped.append({"number": item["number"], "rule": picked})
            continue
        reason = human(picked["_pull"], picked["_commits"])
        if reason:
            skipped.append({"number": item["number"], "rule": reason})
            continue
        return picked, skipped, len(items)
    return None, skipped, len(items)


def public(record):
    return {key: value for key, value in record.items() if not key.startswith("_")}


def main(aidev, clones):
    clones = pathlib.Path(clones)
    clones.mkdir(parents=True, exist_ok=True)
    rows, known = candidates(aidev)
    selection = {"agent": [], "human": [], "skipped": [], "humanSkipped": []}
    for language in EXTENSIONS:
        taken = set()
        kept = 0
        for row in order(rows, language):
            if kept == PER_LANGUAGE:
                break
            pr_id, number, agent_name, _, _, full_name = row
            if full_name.lower() in taken:
                selection["skipped"].append({"id": pr_id, "fullName": full_name, "number": number, "rule": "repository already taken"})
                continue
            picked = eligible(clones, language, full_name, number)
            if isinstance(picked, str):
                selection["skipped"].append({"id": pr_id, "fullName": full_name, "number": number, "rule": picked})
                print(f"skip {language} {full_name}#{number}: {picked}", flush=True)
                continue
            taken.add(full_name.lower())
            taken.add(picked["fullName"].lower())
            picked = {"id": pr_id, "agent": agent_name, **public(picked)}
            selection["agent"].append(picked)
            kept += 1
            print(f"agent {language} {agent_name} {picked['fullName']}#{number}", flush=True)
            mate, skipped, seen = human_for(clones, known, picked)
            selection["humanSkipped"].append({"fullName": picked["fullName"], "agentNumber": number, "searched": seen, "skipped": skipped})
            if mate:
                selection["human"].append({"agentNumber": number, **public(mate)})
                print(f"human {language} {mate['fullName']}#{mate['number']}", flush=True)
            else:
                print(f"human {language} {picked['fullName']}: none eligible", flush=True)
    (HERE / "selection.json").write_text(json.dumps(selection, indent=2, ensure_ascii=False) + "\n")


if __name__ == "__main__":
    main(*sys.argv[1:])
