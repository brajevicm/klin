#!/usr/bin/env python3
"""Materialize and replay the frozen natural phenotype sample.

Run with:
  uv run --with duckdb python study.py materialize --cache /path/to/cache
  uv run --with duckdb python study.py replay --cache /path/to/cache
"""

from __future__ import annotations

import argparse
import csv
import datetime as dt
import hashlib
import json
import math
import os
import pathlib
import re
import shutil
import subprocess
import sys
import time
import urllib.error
import urllib.parse
import urllib.request

HERE = pathlib.Path(__file__).resolve().parent
AIDEV_REVISION = "c63c8a57a2de34fc03fa83722412824af4d8753b"
AIDEV_FILES = {
    "pull_request.parquet": "c0b8e81e1d099905ef9ea420bf907d45179771ff5972afce6c219d8cafcef3e8",
    "repository.parquet": "a08e34be4921c708be88a4ebd9e275b32f37fd442bb2770c0b69c94834dc6aa7",
}
LANGUAGES = {
    "Rust": (".rs",),
    "TypeScript": (".ts", ".tsx", ".mts", ".cts"),
    "Python": (".py", ".pyi"),
}
AGENTS = ("Claude_Code", "OpenAI_Codex", "Cursor", "Copilot", "Devin", "Google_Jules")
AGENT_BRANCHES = ("codex/", "cursor/", "copilot/", "devin/", "claude/", "jules/", "jules-")
AGENT_WORDS = re.compile(
    r"claude code|co-authored-by: claude|codex|cursor agent|cursoragent|devin|jules|copilot|generated with|\U0001f916",
    re.IGNORECASE,
)
DATE_FROM = dt.date(2024, 12, 24)
DATE_TO = dt.date(2025, 10, 24)
MAX_KB = 150_000
MAX_FILES = 100
HUMAN_WINDOW_DAYS = 60
TARGET_PER_LANGUAGE = 40
API_BLOCKED = {"_study_api_state": "repository-access-blocked"}


class StudyError(RuntimeError):
    pass


def sha256(path: pathlib.Path) -> str:
    digest = hashlib.sha256()
    with path.open("rb") as source:
        for block in iter(lambda: source.read(1024 * 1024), b""):
            digest.update(block)
    return digest.hexdigest()


def download_inputs(data_dir: pathlib.Path) -> dict[str, pathlib.Path]:
    data_dir.mkdir(parents=True, exist_ok=True)
    paths = {}
    for name, expected in AIDEV_FILES.items():
        path = data_dir / name
        if not path.is_file() or sha256(path) != expected:
            url = (
                "https://huggingface.co/datasets/hao-li/AIDev/resolve/"
                f"{AIDEV_REVISION}/{name}"
            )
            temporary = path.with_suffix(path.suffix + ".partial")
            request = urllib.request.Request(
                url, headers={"User-Agent": "klin-phenotype-study-457"}
            )
            digest = hashlib.sha256()
            try:
                with urllib.request.urlopen(request, timeout=60) as response:
                    with temporary.open("wb") as target:
                        while block := response.read(1024 * 1024):
                            digest.update(block)
                            target.write(block)
            except (OSError, urllib.error.URLError) as error:
                temporary.unlink(missing_ok=True)
                raise StudyError(f"could not download frozen AIDev input {name}: {error}")
            if digest.hexdigest() != expected:
                temporary.unlink(missing_ok=True)
                raise StudyError(f"AIDev input hash mismatch for {name}")
            temporary.replace(path)
        paths[name] = path
    return paths


class GitHub:
    def __init__(self, cache_dir: pathlib.Path | None = None) -> None:
        self.cache: dict[str, object] = {}
        self.cache_dir = cache_dir
        if cache_dir is not None:
            cache_dir.mkdir(parents=True, exist_ok=True)

    def get(self, path: str, *, paginate: bool = False) -> object | None:
        key = ("pages:" if paginate else "one:") + path
        if key in self.cache:
            return self.cache[key]
        cached = self.cache_dir / (hashlib.sha256(key.encode()).hexdigest() + ".json") if self.cache_dir else None
        if cached is not None and cached.is_file():
            value = json.loads(cached.read_text())
            self.cache[key] = value
            return value
        args = ["gh", "api"]
        if paginate:
            args += ["--paginate", "--slurp"]
        args.append(path)
        result = None
        for attempt in range(5):
            try:
                result = subprocess.run(args, capture_output=True, text=True, timeout=90)
            except subprocess.TimeoutExpired:
                if attempt == 4:
                    raise StudyError(f"gh api {path} timed out after retries")
                time.sleep(2 ** attempt)
                continue
            transient = any(text in result.stderr.lower() for text in (
                "connection reset", "connection refused", "could not resolve host", "timed out",
                "remote end hung up", "the requested url returned error: 502", "http 502", "http 503", "http 504",
            ))
            if result.returncode and transient and attempt < 4:
                time.sleep(2 ** attempt)
                continue
            break
        assert result is not None
        if result.returncode:
            if "HTTP 404" in result.stderr or "Not Found" in result.stderr:
                self.cache[key] = None
                return None
            if "Repository access blocked" in result.stderr:
                self.cache[key] = API_BLOCKED
                if cached is not None:
                    cached.write_text(json.dumps(API_BLOCKED))
                return API_BLOCKED
            raise StudyError(f"gh api {path} failed: {result.stderr.strip()}")
        try:
            value = json.loads(result.stdout)
        except json.JSONDecodeError as error:
            raise StudyError(f"gh api {path} returned invalid JSON: {error}")
        self.cache[key] = value
        if cached is not None:
            cached.write_text(json.dumps(value, separators=(",", ":")))
        return value

    def changed_file_counts(
        self, repository: str, pulls: list[dict[str, object]]
    ) -> list[dict[str, object]]:
        output = list(pulls)
        missing = [item for item in output if not isinstance(item.get("changed_files"), int)]
        for offset in range(0, len(missing), 50):
            batch = missing[offset:offset + 50]
            query = "query { " + " ".join(
                f"p{index}: node(id: {json.dumps(item['node_id'])}) "
                "{ ... on PullRequest { changedFiles } }"
                for index, item in enumerate(batch) if item.get("node_id")
            ) + " }"
            counts: dict[str, object] = {}
            if query != "query {  }":
                cache_key = "graphql:" + query
                cached = self.cache_dir / (hashlib.sha256(cache_key.encode()).hexdigest() + ".json") if self.cache_dir else None
                if cached is not None and cached.is_file():
                    response = json.loads(cached.read_text(encoding="utf-8"))
                else:
                    result = subprocess.run(
                        ["gh", "api", "graphql", "-f", f"query={query}"],
                        capture_output=True, text=True, timeout=90,
                    )
                    if result.returncode:
                        raise StudyError(f"gh api graphql failed while reading pull-request file counts: {result.stderr.strip()}")
                    try:
                        response = json.loads(result.stdout)
                    except json.JSONDecodeError as error:
                        raise StudyError(f"gh api graphql returned invalid JSON: {error}")
                    if cached is not None:
                        cached.write_text(json.dumps(response, separators=(",", ":")))
                if response.get("errors"):
                    raise StudyError(f"GitHub GraphQL could not read pull-request file counts: {response['errors']}")
                counts = response.get("data") or {}
            for index, item in enumerate(batch):
                result = counts.get(f"p{index}") or {}
                count = result.get("changedFiles")
                if not isinstance(count, int):
                    detail = self.get(f"repos/{repository}/pulls/{item['number']}")
                    count = detail.get("changed_files") if isinstance(detail, dict) else None
                if not isinstance(count, int):
                    raise StudyError(f"changed-file count is unavailable for {repository}#{item['number']}")
                item["changed_files"] = count
        return output


def git(args: list[str], *, timeout: int = 180) -> subprocess.CompletedProcess[str]:
    return subprocess.run(
        ["git", *args],
        capture_output=True,
        text=True,
        timeout=timeout,
    )


def git_retry(args: list[str], *, timeout: int = 180) -> subprocess.CompletedProcess[str]:
    result = None
    for attempt in range(5):
        try:
            result = git(args, timeout=timeout)
        except subprocess.TimeoutExpired:
            if attempt == 4:
                raise
            time.sleep(2 ** attempt)
            continue
        transient = any(text in result.stderr.lower() for text in (
            "connection reset", "connection refused", "could not resolve host", "timed out",
            "remote end hung up", "the remote end hung up unexpectedly", "http/2 stream",
            "the requested url returned error: 502", "the requested url returned error: 503",
        ))
        if result.returncode and transient and attempt < 4:
            time.sleep(2 ** attempt)
            continue
        return result
    assert result is not None
    return result


class GitFetch:
    """Fetch exact PR commits into filtered bare repositories for eligibility checks."""

    def __init__(self, root: pathlib.Path) -> None:
        self.root = root
        self.root.mkdir(parents=True, exist_ok=True)

    def merge_base(
        self, repository: str, number: int, base: str, head: str, base_ref: str,
        merge_commit: str = "", first_commit_parent: str = "",
    ) -> tuple[str | None, str | None]:
        bare = self.root / (repository.lower().replace("/", "__") + ".git")
        if not bare.exists():
            initialized = git(["init", "--bare", "--quiet", str(bare)])
            if initialized.returncode:
                return None, initialized.stderr.strip() or "git init failed"
            added = git(["--git-dir", str(bare), "remote", "add", "origin", f"https://github.com/{repository}.git"])
            if added.returncode:
                return None, added.stderr.strip() or "git remote add failed"

        pull_ref = f"+refs/pull/{number}/head:refs/study/pull/{number}"
        base_branch = "refs/study/base/" + str(number)
        for depth in (64, 256, 1024, None):
            fetch_args = ["--git-dir", str(bare), "fetch", "--quiet", "--filter=blob:none", "--no-tags"]
            if depth is not None:
                fetch_args.append(f"--depth={depth}")
            fetch_args += ["origin", pull_ref]
            fetched = git_retry(fetch_args, timeout=600)
            if fetched.returncode:
                return None, "GitHub PR head could not be fetched: " + fetched.stderr.strip()
            fetched_head = git(["--git-dir", str(bare), "rev-parse", f"refs/study/pull/{number}"]).stdout.strip()
            if fetched_head != head:
                return None, "fetched PR head differs from the frozen GitHub API head"

            target_base = base
            if re.fullmatch(r"[0-9a-f]{40}", merge_commit):
                merge_args = ["--git-dir", str(bare), "fetch", "--quiet", "--filter=blob:none", "--no-tags"]
                if depth is not None:
                    merge_args.append(f"--depth={depth}")
                merge_args += ["origin", merge_commit]
                merged = git_retry(merge_args, timeout=600)
                if merged.returncode == 0:
                    parents = git(["--git-dir", str(bare), "rev-list", "--parents", "-n", "1", merge_commit]).stdout.split()
                    if len(parents) >= 3:
                        target_base = parents[1]

            base_args = ["--git-dir", str(bare), "fetch", "--quiet", "--filter=blob:none", "--no-tags"]
            if depth is not None:
                base_args.append(f"--depth={depth}")
            base_args += ["origin", target_base]
            fetched_base = git_retry(base_args, timeout=600)
            if fetched_base.returncode:
                branch_args = ["--git-dir", str(bare), "fetch", "--quiet", "--filter=blob:none", "--no-tags"]
                if depth is not None:
                    branch_args.append(f"--depth={depth}")
                branch_args += ["origin", f"+refs/heads/{base_ref}:{base_branch}"]
                fetched_base = git_retry(branch_args, timeout=600)
            have_base = git(["--git-dir", str(bare), "cat-file", "-e", f"{target_base}^{{commit}}"])
            if have_base.returncode == 0:
                found = git(["--git-dir", str(bare), "merge-base", target_base, head])
                if found.returncode == 0 and found.stdout.strip() and found.stdout.strip() != head:
                    return found.stdout.strip(), None
                if first_commit_parent and first_commit_parent != head:
                    first_args = ["--git-dir", str(bare), "fetch", "--quiet", "--filter=blob:none", "--no-tags"]
                    if depth is not None:
                        first_args.append(f"--depth={depth}")
                    first_args += ["origin", first_commit_parent]
                    git_retry(first_args, timeout=600)
                    recovered = git(["--git-dir", str(bare), "merge-base", first_commit_parent, head])
                    if recovered.returncode == 0 and recovered.stdout.strip() and recovered.stdout.strip() != head:
                        return recovered.stdout.strip(), None
        return None, "GitHub commits were fetched but no merge base was available"


def date_value(value: str) -> dt.datetime:
    return dt.datetime.fromisoformat(value.replace("Z", "+00:00"))


def repository_id(
    repository: str, number: int, pull: dict[str, object]
) -> str:
    return str(pull.get("id", f"{repository}#{number}"))


def pr_files(api: GitHub, repository: str, number: int) -> list[dict[str, object]] | None:
    value = api.get(f"repos/{repository}/pulls/{number}/files?per_page=100")
    return value if isinstance(value, list) else None


def touches_language(files: list[dict[str, object]], language: str) -> bool:
    extensions = LANGUAGES[language]
    for item in files:
        names = [str(item.get("filename", "")), str(item.get("previous_filename", ""))]
        if any(name.endswith(extensions) for name in names):
            return True
    return False


def eligibility(
    api: GitHub,
    fetcher: GitFetch,
    repo_cache: dict[str, dict[str, object] | None],
    repository: str,
    number: int,
    language: str,
    *,
    pull: dict[str, object] | None = None,
) -> tuple[dict[str, object] | None, str]:
    key = repository.lower()
    if key not in repo_cache:
        value = api.get("repos/" + repository)
        repo_cache[key] = value if isinstance(value, dict) else None
    repo = repo_cache[key]
    if repo is None:
        return None, "repository not readable"
    if repo.get("fork") is True:
        return None, "repository is a fork"
    size = int(repo.get("size", MAX_KB + 1))
    if size > MAX_KB:
        return None, f"repository size {size} KB exceeds {MAX_KB} KB"

    if pull is None:
        value = api.get(f"repos/{repository}/pulls/{number}")
        pull = value if isinstance(value, dict) else None
    if pull is None:
        return None, "pull request not readable"
    if pull.get("state") != "closed" or not pull.get("closed_at"):
        return None, "pull request is not closed"
    changed_files = int(pull.get("changed_files", MAX_FILES + 1))
    if changed_files > MAX_FILES:
        return None, f"{changed_files} changed files exceeds {MAX_FILES}"
    base = pull.get("base") or {}
    head = pull.get("head") or {}
    base_sha = str(base.get("sha", ""))
    head_sha = str(head.get("sha", ""))
    if not re.fullmatch(r"[0-9a-f]{40}", base_sha) or not re.fullmatch(r"[0-9a-f]{40}", head_sha):
        return None, "base or head SHA is unavailable"

    files = pr_files(api, repository, number)
    if files is None:
        return None, "changed-file list is unavailable"
    if len(files) != changed_files:
        return None, f"changed-file list has {len(files)} entries; API reports {changed_files}"
    if not touches_language(files, language):
        return None, "no changed file has the study language"

    config = api.get(f"repos/{repository}/contents/klin.json?ref={head_sha}")
    if config == API_BLOCKED:
        return None, "root klin.json presence could not be checked"
    if config is not None:
        return None, "root klin.json exists at head"

    merge_commit = str(pull.get("merge_commit_sha") or "")
    first_commit_parent = ""
    if pull.get("merged_at") and merge_commit == head_sha:
        commit_rows = commits(api, repository, number)
        if commit_rows:
            parents = commit_rows[0].get("parents") or (commit_rows[0].get("commit") or {}).get("parents") or []
            if parents:
                first_commit_parent = str(parents[0].get("sha", ""))
    merge, failure = fetcher.merge_base(
        repository, number, base_sha, head_sha, str(base.get("ref", "")),
        merge_commit, first_commit_parent,
    )
    if merge is None:
        return None, failure or "no merge base"

    return {
        "repository": str(repo.get("full_name", repository)),
        "number": number,
        "url": str(pull.get("html_url", "")),
        "created_at": str(pull.get("created_at", "")),
        "closed_at": str(pull.get("closed_at", "")),
        "merged_at": str(pull.get("merged_at") or ""),
        "merge_state": "merged" if pull.get("merged_at") else "closed-unmerged",
        "author_login": str((pull.get("user") or {}).get("login", "")),
        "author_type": str((pull.get("user") or {}).get("type", "")),
        "head_branch": str(head.get("ref", "")),
        "base": merge,
        "head": head_sha,
        "changed_files": changed_files,
        "changed_language_files": sum(
            1 for item in files
            if any(
                str(item.get(field, "")).endswith(LANGUAGES[language])
                for field in ("filename", "previous_filename")
            )
        ),
        "title": str(pull.get("title", "")),
        "body": str(pull.get("body") or ""),
        "repository_size_kb": size,
        "pull_id": repository_id(repository, number, pull),
    }, ""


def pilot_repositories() -> set[str]:
    path = HERE.parent / "phenotype-pilot-2026-10-02" / "selection.json"
    sample = json.loads(path.read_text())
    return {str(row["fullName"]).lower() for row in sample.get("agent", [])}


def ordered_candidates(rows: list[tuple[object, ...]]):
    groups = {agent: [] for agent in AGENTS}
    for row in rows:
        groups[str(row[2])].append(row)
    for agent in AGENTS:
        groups[agent].sort(
            key=lambda row: hashlib.sha256(f"357-final-v1:{row[0]}".encode()).hexdigest()
        )
    while any(groups.values()):
        for agent in AGENTS:
            if groups[agent]:
                yield groups[agent].pop(0)


def aidv_rows(data: dict[str, pathlib.Path]) -> tuple[list[tuple[object, ...]], dict[str, set[int]]]:
    try:
        import duckdb
    except ImportError as error:
        raise StudyError("DuckDB is required; run with uv run --with duckdb") from error
    pr = str(data["pull_request.parquet"]).replace("'", "''")
    repos = str(data["repository.parquet"]).replace("'", "''")
    db = duckdb.connect()
    candidates = db.execute(
        f"""
        SELECT p.id, p.number, p.agent, p.created_at, p.repo_id, r.full_name
        FROM read_parquet('{pr}') p JOIN read_parquet('{repos}') r ON p.repo_id = r.id
        WHERE p.agent IN ({','.join(repr(agent) for agent in AGENTS)})
          AND p.closed_at IS NOT NULL AND NOT r.is_forked
          AND CAST(p.created_at AS DATE) BETWEEN DATE '{DATE_FROM}' AND DATE '{DATE_TO}'
        """
    ).fetchall()
    known = db.execute(
        f"""
        SELECT r.full_name, p.number
        FROM read_parquet('{pr}') p JOIN read_parquet('{repos}') r ON p.repo_id = r.id
        """
    ).fetchall()
    by_repo: dict[str, set[int]] = {}
    for name, number in known:
        by_repo.setdefault(str(name).lower(), set()).add(int(number))
    db.close()
    return candidates, by_repo


def pull_pages(api: GitHub, repository: str, low: dt.datetime, high: dt.datetime):
    page = 1
    while True:
        path = (
            f"repos/{repository}/pulls?state=closed&sort=created&direction=desc"
            f"&per_page=100&page={page}"
        )
        result = api.get(path)
        if not isinstance(result, list) or not result:
            return
        stop = False
        for pull in result:
            created = date_value(str(pull["created_at"]))
            if created < low:
                stop = True
                break
            if created <= high:
                yield pull
        if stop or len(result) < 100:
            return
        page += 1


def commits(api: GitHub, repository: str, number: int) -> list[dict[str, object]] | None:
    pages = api.get(f"repos/{repository}/pulls/{number}/commits?per_page=100", paginate=True)
    if pages == API_BLOCKED:
        return None
    if not isinstance(pages, list):
        return None
    return [commit for page in pages if isinstance(page, list) for commit in page]


def human_exclusion(
    pull: dict[str, object],
    commit_rows: list[dict[str, object]] | None,
) -> str:
    user = pull.get("user") or {}
    login = str(user.get("login", ""))
    if user.get("type") == "Bot" or login.endswith("[bot]") or login == "Copilot":
        return "author is bot or Copilot"
    branch = str((pull.get("head") or {}).get("ref", ""))
    if branch.startswith(AGENT_BRANCHES):
        return "agent branch prefix"
    if commit_rows is None:
        return "commit history is unavailable"
    text = [str(pull.get("body") or "")]
    text.extend(
        str((row.get("commit") or {}).get("message", ""))
        for row in commit_rows
    )
    if any(AGENT_WORDS.search(value) for value in text):
        return "agent marker in body or commit message"
    return ""


def human_match(
    api: GitHub,
    fetcher: GitFetch,
    repo_cache: dict[str, dict[str, object] | None],
    agent: dict[str, object],
    known_numbers: set[int],
) -> tuple[dict[str, object] | None, list[dict[str, object]]]:
    created = date_value(str(agent["created_at"]))
    low, high = created - dt.timedelta(days=HUMAN_WINDOW_DAYS), created + dt.timedelta(days=HUMAN_WINDOW_DAYS)
    repository = str(agent["repository"])
    candidates = list(pull_pages(api, repository, low, high))
    audit: list[dict[str, object]] = []
    possible = []
    for order, item in enumerate(candidates, 1):
        number = int(item.get("number", -1))
        base_row = {
            "language": agent["language"], "candidate_kind": "matched-human",
            "agent_change_id": agent["change_id"], "candidate_order": order,
            "repository": repository, "number": number,
            "pull_id": item.get("id", ""), "created_at": item.get("created_at", ""),
            "changed_files": item.get("changed_files", ""),
            "merge_state": "merged" if item.get("merged_at") else "closed-unmerged",
        }
        if number == int(agent["number"]) or number in known_numbers:
            audit.append({**base_row, "selected": "false", "reason": "PR id is in AIDev pull_request"})
            continue
        user = item.get("user") or {}
        login = str(user.get("login", ""))
        branch = str((item.get("head") or {}).get("ref", ""))
        if user.get("type") == "Bot" or login.endswith("[bot]") or login == "Copilot":
            audit.append({**base_row, "selected": "false", "reason": "author is bot or Copilot"})
            continue
        if branch.startswith(AGENT_BRANCHES):
            audit.append({**base_row, "selected": "false", "reason": "agent branch prefix"})
            continue
        if AGENT_WORDS.search(str(item.get("body") or "")):
            audit.append({**base_row, "selected": "false", "reason": "agent marker in body"})
            continue
        possible.append(item)
    candidates = api.changed_file_counts(repository, possible)
    agent_files = int(agent["changed_files"])
    minimum = math.ceil(agent_files / 2)
    maximum = min(MAX_FILES, 2 * agent_files)
    in_range = []
    for item in candidates:
        files = int(item.get("changed_files", MAX_FILES + 1))
        if minimum <= files <= maximum and files <= MAX_FILES:
            in_range.append(item)
        else:
            audit.append({
                "language": agent["language"], "candidate_kind": "matched-human",
                "agent_change_id": agent["change_id"], "candidate_order": "",
                "repository": repository, "number": item.get("number", ""),
                "pull_id": item.get("id", ""), "created_at": item.get("created_at", ""),
                "changed_files": files,
                "merge_state": "merged" if item.get("merged_at") else "closed-unmerged",
                "selected": "false", "reason": f"{files} changed files outside matched range {minimum}-{maximum}",
            })
    candidates = in_range
    candidates.sort(
        key=lambda item: (
            0 if bool(item.get("merged_at")) == (agent["merge_state"] == "merged") else 1,
            abs(math.log2(int(item["changed_files"]) / agent_files)),
            abs((date_value(str(item["created_at"])) - created).total_seconds()),
            int(item["number"]),
        )
    )
    for rank, item in enumerate(candidates, 1):
        number = int(item["number"])
        base_row = {
            "language": agent["language"],
            "candidate_kind": "matched-human",
            "agent_change_id": agent["change_id"],
            "candidate_order": rank,
            "repository": repository,
            "number": number,
            "pull_id": item.get("id", ""),
            "created_at": item.get("created_at", ""),
            "changed_files": item.get("changed_files", ""),
            "merge_state": "merged" if item.get("merged_at") else "closed-unmerged",
        }
        reason = human_exclusion(item, commits(api, repository, number))
        if reason:
            audit.append({**base_row, "selected": "false", "reason": reason})
            continue
        picked, reason = eligibility(
            api, fetcher, repo_cache, repository, number, str(agent["language"]), pull=item
        )
        if picked is None:
            audit.append({**base_row, "selected": "false", "reason": reason})
            continue
        picked.update({
            "change_id": f"human-{str(agent['language']).lower()}-{agent['aidv_id']}-{picked['pull_id']}",
            "population": "matched-human",
            "paired_agent_change_id": agent["change_id"],
            "language": agent["language"],
            "provenance": "GitHub PR not attributed to an agent by the frozen metadata exclusions",
            "agent": "",
            "aidv_id": "",
            "selection_stratum": f"matched-human:{agent['language']}",
            "selection_order": "",
            "match_status": "matched",
            "match_candidate_order": rank,
            "match_merge_state": "same" if picked["merge_state"] == agent["merge_state"] else "different",
            "match_file_ratio_distance": abs(math.log2(picked["changed_files"] / agent_files)),
            "match_time_distance_days": abs(
                (date_value(picked["created_at"]) - created).total_seconds()
            ) / 86400,
        })
        audit.append({**base_row, "selected": "true", "reason": "first eligible candidate in frozen order"})
        return picked, audit
    audit.append({
        "language": agent["language"], "candidate_kind": "matched-human",
        "agent_change_id": agent["change_id"], "candidate_order": "",
        "repository": repository, "number": "", "pull_id": "", "created_at": "",
        "changed_files": "", "merge_state": "", "selected": "false",
        "reason": "no eligible human PR in the frozen match window",
    })
    return None, audit


SAMPLE_FIELDS = (
    "change_id", "population", "paired_agent_change_id", "repository", "base", "head",
    "language", "provenance", "agent", "aidv_id", "pull_id", "number", "url",
    "title", "body", "created_at", "closed_at", "merge_state", "head_branch",
    "author_login", "author_type", "changed_files", "changed_language_files",
    "repository_size_kb", "selection_stratum", "selection_order", "match_status",
    "match_candidate_order", "match_merge_state", "match_file_ratio_distance",
    "match_time_distance_days",
)
AUDIT_FIELDS = (
    "language", "candidate_kind", "agent_change_id", "candidate_order", "repository",
    "number", "pull_id", "created_at", "changed_files", "merge_state", "selected", "reason",
)

GATE_BY_PHENOTYPE = {
    "shipped-complexity": "complexity",
    "shipped-escapes": "escapes",
    "shipped-stubs": "stubs",
    "shipped-test-deletion": "inventory",
    "shipped-test-skip": "escapes",
    "shipped-dead-symbols": "dead-symbols",
    "shipped-reachability": "reachability",
    "shipped-lockfile": "lockfile",
    "shipped-module-cycle": "layering",
    "shipped-public-api": "public-api",
}
ASSERT_CANDIDATES = {
    "test-all-checks-removed": {"all-checks-removed"},
    "test-weakened-rust": {"assertion-removed"},
    "test-disabled": {"disabled"},
    "test-expected-mirrors-production": {"expected-changed"},
    "test-new-unchecked": {"new-test-unchecked"},
}
SHAPE_CANDIDATES = {
    "unfinished-ellipsis-body": "ellipsis-body",
    "unfinished-throw-body": "throw-body",
    "error-empty-handler": "empty-handler",
    "error-default-handler": "default-handler",
    "error-broad-handler": "broad-handler",
}
RELATION_CANDIDATES = {
    "design-family-bypass": "family-bypass",
    "design-registration-bypass": "registration-bypass",
    "design-wrapper-bypass": "wrapper-bypass",
    "design-component-cycle": "component-cycle",
}
SOURCE_SUFFIXES = (
    ".go", ".java", ".kt", ".kts", ".py", ".rb", ".rs", ".sh", ".bash",
    ".zsh", ".swift", ".ts", ".tsx", ".mts", ".cts", ".js", ".jsx", ".mjs", ".cjs",
)
SKIP_SEGMENTS = {
    ".git", "node_modules", "vendor", "build", ".build", "dist", "target", "__pycache__",
    ".venv", "venv", "DerivedData", "Pods", "coverage", ".next", "out", "fixtures",
}
MEASUREMENT_FIELDS = (
    "row_id", "study_version", "change_id", "population_id", "repository", "base", "head",
    "language", "phenotype_id", "semantic_eligible", "eligibility_count", "measurement_state",
    "measured_count", "runtime_ms", "measurement_basis", "holes", "values",
)
FINDING_FIELDS = (
    "finding_row_id", "study_version", "measurement_row_id", "source_finding_id", "finding_id",
    "finding_site", "evidence_packet_id",
)


def write_tsv(path: pathlib.Path, fields: tuple[str, ...], rows: list[dict[str, object]]) -> None:
    with path.open("w", encoding="utf-8", newline="") as target:
        writer = csv.DictWriter(
            target, fieldnames=fields, delimiter="\t", lineterminator="\n", extrasaction="ignore"
        )
        writer.writeheader()
        writer.writerows(rows)


def materialize(cache: pathlib.Path) -> None:
    data = download_inputs(cache / "aidev")
    rows, known = aidv_rows(data)
    pilot = pilot_repositories()
    api, fetcher = GitHub(cache / "api-cache"), GitFetch(cache / "eligibility-git")
    repo_cache: dict[str, dict[str, object] | None] = {}
    checkpoint_path = cache / "selection-progress.json"
    checkpoint = json.loads(checkpoint_path.read_text()) if checkpoint_path.is_file() else {}
    samples: list[dict[str, object]] = checkpoint.get("samples", [])
    audits: list[dict[str, object]] = checkpoint.get("audits", [])
    processed: set[str] = set(checkpoint.get("processed", []))
    selected_repositories = {
        str(row["repository"]).lower() for row in samples if row["population"] == "natural-agent"
    }
    selected_heads = {
        str(row["head"]) for row in samples if row["population"] == "natural-agent"
    }

    def save_checkpoint() -> None:
        checkpoint_path.write_text(json.dumps({
            "samples": samples, "audits": audits, "processed": sorted(processed),
        }, indent=2) + "\n")

    for language in LANGUAGES:
        taken = sum(
            row["population"] == "natural-agent" and row["language"] == language for row in samples
        )
        walked = 0
        for aidv_id, number_value, agent, created_at, _repo_id, name in ordered_candidates(rows):
            if taken >= TARGET_PER_LANGUAGE:
                break
            walked += 1
            candidate_key = f"{language}:{aidv_id}"
            if candidate_key in processed:
                continue
            if walked % 100 == 0:
                print(
                    f"checked {language} {walked} ordered agent candidates; "
                    f"{taken}/{TARGET_PER_LANGUAGE} selected",
                    flush=True,
                )
            repository = str(name)
            key = repository.lower()
            base_audit = {
                "language": language, "candidate_kind": "natural-agent",
                "agent_change_id": "", "candidate_order": walked,
                "repository": repository, "number": int(number_value),
                "pull_id": str(aidv_id), "created_at": str(created_at),
            }
            if key in pilot:
                audits.append({**base_audit, "selected": "false", "reason": "repository used by 2026-10-02 pilot agent arm"})
                processed.add(candidate_key)
                save_checkpoint()
                continue
            if key in selected_repositories:
                audits.append({**base_audit, "selected": "false", "reason": "repository cap already used"})
                processed.add(candidate_key)
                save_checkpoint()
                continue
            value = api.get(f"repos/{repository}/pulls/{int(number_value)}")
            pull = value if isinstance(value, dict) else None
            if pull is not None and str(pull.get("head", {}).get("sha", "")) in selected_heads:
                audits.append({**base_audit, "selected": "false", "reason": "head SHA already selected"})
                processed.add(candidate_key)
                save_checkpoint()
                continue
            picked, reason = eligibility(
                api, fetcher, repo_cache, repository, int(number_value), language, pull=pull
            )
            if picked is None:
                audits.append({**base_audit, "selected": "false", "reason": reason})
                processed.add(candidate_key)
                save_checkpoint()
                continue
            if picked["head"] in selected_heads:
                audits.append({**base_audit, "selected": "false", "reason": "head SHA already selected"})
                processed.add(candidate_key)
                save_checkpoint()
                continue
            change_id = f"agent-{language.lower()}-{aidv_id}"
            picked.update({
                "change_id": change_id,
                "population": "natural-agent",
                "paired_agent_change_id": "",
                "language": language,
                "provenance": f"AIDev v4:{agent}",
                "agent": str(agent),
                "aidv_id": str(aidv_id),
                "selection_stratum": f"natural-agent:{language}:{agent}",
                "selection_order": taken + 1,
                "match_status": "unmatched",
                "match_candidate_order": "",
                "match_merge_state": "",
                "match_file_ratio_distance": "",
                "match_time_distance_days": "",
            })
            audits.append({**base_audit, "selected": "true", "reason": "first eligible candidate in frozen order"})
            selected_repositories.add(key)
            selected_heads.add(str(picked["head"]))
            samples.append(picked)
            taken += 1
            print(f"agent {language} {taken}/{TARGET_PER_LANGUAGE}: {repository}#{number_value}", flush=True)

            mate, match_audit = human_match(
                api, fetcher, repo_cache, picked, known.get(key, set())
            )
            audits.extend(match_audit)
            if mate is not None:
                picked["match_status"] = "matched"
                samples.append(mate)
                print(f"human {language}: {repository}#{mate['number']}", flush=True)
            else:
                print(f"human {language}: no eligible match", flush=True)
            processed.add(candidate_key)
            save_checkpoint()
        if taken < TARGET_PER_LANGUAGE:
            print(f"population has {taken} eligible {language} agent changes; target is {TARGET_PER_LANGUAGE}", flush=True)

    write_tsv(HERE / "natural-sample.tsv", SAMPLE_FIELDS, samples)
    write_tsv(HERE / "selection-audit.tsv", AUDIT_FIELDS, audits)
    manifest = {
        "study_version": 2,
        "study_commit": "138dc8d0a927c60df289bd485627f472488cf2ba",
        "aidev_revision": AIDEV_REVISION,
        "aidev_sha256": AIDEV_FILES,
        "selection_rule": "docs/phenotype-study-2026-10-04-v2/populations.tsv",
        "agent_target_per_language": TARGET_PER_LANGUAGE,
        "agent_counts": {
            language: sum(row["population"] == "natural-agent" and row["language"] == language for row in samples)
            for language in LANGUAGES
        },
        "matched_human_counts": {
            language: sum(row["population"] == "matched-human" and row["language"] == language for row in samples)
            for language in LANGUAGES
        },
        "unmatched_agent_change_ids": [
            row["change_id"] for row in samples
            if row["population"] == "natural-agent" and row["match_status"] != "matched"
        ],
    }
    (HERE / "study-inputs.json").write_text(json.dumps(manifest, indent=2) + "\n")
    print(json.dumps(manifest, indent=2))


def rematch_humans(cache: pathlib.Path) -> None:
    checkpoint_path = cache / "selection-progress.json"
    if not checkpoint_path.is_file():
        raise StudyError("selection checkpoint is missing; run materialize first")
    data = download_inputs(cache / "aidev")
    _rows, known = aidv_rows(data)
    checkpoint = json.loads(checkpoint_path.read_text(encoding="utf-8"))
    samples: list[dict[str, object]] = checkpoint.get("samples", [])
    audits: list[dict[str, object]] = checkpoint.get("audits", [])
    api = GitHub(cache / "api-cache")
    fetcher = GitFetch(cache / "eligibility-git")
    repo_cache: dict[str, dict[str, object] | None] = {}
    paired = {
        str(row["paired_agent_change_id"])
        for row in samples if row["population"] == "matched-human"
    }
    for agent in [row for row in samples if row["population"] == "natural-agent"]:
        if str(agent["change_id"]) in paired:
            continue
        mate, match_audit = human_match(
            api, fetcher, repo_cache, agent,
            known.get(str(agent["repository"]).lower(), set()),
        )
        audits.extend(match_audit)
        if mate is None:
            print(
                f"human {agent['language']}: no eligible match for "
                f"{agent['repository']}#{agent['number']}",
                flush=True,
            )
            continue
        agent["match_status"] = "matched"
        samples.append(mate)
        paired.add(str(agent["change_id"]))
        print(f"human {agent['language']}: {agent['repository']}#{mate['number']}", flush=True)

    checkpoint_path.write_text(json.dumps({
        "samples": samples, "audits": audits, "processed": checkpoint.get("processed", []),
    }, indent=2) + "\n", encoding="utf-8")
    write_tsv(HERE / "natural-sample.tsv", SAMPLE_FIELDS, samples)
    write_tsv(HERE / "selection-audit.tsv", AUDIT_FIELDS, audits)
    manifest = {
        "study_version": 2,
        "study_commit": "138dc8d0a927c60df289bd485627f472488cf2ba",
        "aidev_revision": AIDEV_REVISION,
        "aidev_sha256": AIDEV_FILES,
        "selection_rule": "docs/phenotype-study-2026-10-04-v2/populations.tsv",
        "agent_target_per_language": TARGET_PER_LANGUAGE,
        "agent_counts": {
            language: sum(row["population"] == "natural-agent" and row["language"] == language for row in samples)
            for language in LANGUAGES
        },
        "matched_human_counts": {
            language: sum(row["population"] == "matched-human" and row["language"] == language for row in samples)
            for language in LANGUAGES
        },
        "unmatched_agent_change_ids": [
            row["change_id"] for row in samples
            if row["population"] == "natural-agent" and row["match_status"] != "matched"
        ],
    }
    (HERE / "study-inputs.json").write_text(json.dumps(manifest, indent=2) + "\n", encoding="utf-8")
    print(json.dumps(manifest, indent=2))


def command(
    args: list[str], *, cwd: pathlib.Path | None = None,
    env: dict[str, str] | None = None, timeout: int = 600,
) -> subprocess.CompletedProcess[str]:
    try:
        return subprocess.run(args, cwd=cwd, env=env, capture_output=True, text=True, timeout=timeout)
    except subprocess.TimeoutExpired as error:
        stdout = error.stdout or ""
        stderr = error.stderr or ""
        if isinstance(stdout, bytes):
            stdout = stdout.decode(errors="replace")
        if isinstance(stderr, bytes):
            stderr = stderr.decode(errors="replace")
        return subprocess.CompletedProcess(
            args, 124, stdout,
            f"{stderr}\ncommand timed out after {timeout} seconds".strip(),
        )


def require(result: subprocess.CompletedProcess[str], purpose: str) -> str:
    if result.returncode:
        raise StudyError(f"{purpose} failed ({result.returncode}): {result.stderr.strip()}")
    return result.stdout


def baseline_source(cache: pathlib.Path) -> pathlib.Path:
    source = cache / "study-v2" / "study-source"
    marker = source / ".study-commit"
    if marker.is_file() and marker.read_text().strip() == "138dc8d0a927c60df289bd485627f472488cf2ba":
        return source
    shutil.rmtree(source, ignore_errors=True)
    source.mkdir(parents=True)
    archive = cache / "study-v2" / "study-source.tar"
    repo = HERE.parents[1]
    result = command([
        "git", "-C", str(repo), "archive", "--format=tar",
        "138dc8d0a927c60df289bd485627f472488cf2ba", "-o", str(archive),
    ])
    require(result, "exporting the frozen study source")
    require(command(["tar", "-xf", str(archive), "-C", str(source)]), "extracting the study source")
    archive.unlink(missing_ok=True)
    marker.write_text("138dc8d0a927c60df289bd485627f472488cf2ba\n")
    return source


def build_tools(cache: pathlib.Path) -> dict[str, object]:
    cache.mkdir(parents=True, exist_ok=True)
    source = baseline_source(cache)
    target = cache / "study-v2" / "tools-target"
    bins: dict[str, pathlib.Path] = {}
    for name, relative in (
        ("asserts", "docs/test-integrity-2026-10-02/prototype/Cargo.toml"),
        ("relations", "docs/design-conformance-2026-10-02/prototype/Cargo.toml"),
        ("shapes", "docs/unfinished-code-2026-10-02/prototype/Cargo.toml"),
    ):
        result = command([
            "cargo", "build", "--locked", "--release", "--manifest-path", str(source / relative),
        ], env={**os.environ, "CARGO_TARGET_DIR": str(target)}, timeout=3600)
        require(result, f"building frozen {name} prototype")
        bins[name] = target / "release" / name
        if not bins[name].is_file():
            raise StudyError(f"frozen {name} prototype binary was not produced")

    uv_env = {
        **os.environ,
        "UV_CACHE_DIR": str(cache / "uv-cache"),
        "UV_TOOL_DIR": str(cache / "uv-tools"),
    }
    ruff_result = command(
        ["uv", "tool", "run", "--from", "ruff==0.16.10", "ruff", "--version"],
        env=uv_env, timeout=600,
    )
    ruff_version = require(ruff_result, "resolving the frozen Ruff tool").strip()
    if ruff_version != "ruff 0.16.10":
        raise StudyError(f"Ruff version mismatch: {ruff_version}")

    klin = target / "klin-study-baseline"
    result = command([
        "cargo", "build", "--locked", "--release", "--manifest-path", str(source / "Cargo.toml"),
        "--bin", "klin",
    ], env={**os.environ, "CARGO_TARGET_DIR": str(klin)}, timeout=3600)
    require(result, "building frozen klin")
    executable = klin / "release" / "klin"
    manifest = {
        "study_commit": "138dc8d0a927c60df289bd485627f472488cf2ba",
        "klin_version": require(command([str(executable), "--version"]), "reading klin version").strip(),
        "klin_sha256": sha256(executable),
        "ruff_version": ruff_version,
        "ruff_recipe": {
            "injection": "S102,S307,S602,S604,S605,S608",
            "swallowed": "BLE001,S110,S112",
            "flags": ["--isolated", "--no-cache", "--ignore-noqa"],
        },
        "prototypes": {name: sha256(path) for name, path in bins.items()},
    }
    (HERE / "tool-manifest.json").write_text(json.dumps(manifest, indent=2) + "\n")
    print(json.dumps(manifest, indent=2))
    return {"source": source, "klin": executable, "bins": bins, "ruff_env": uv_env}


def bare_repository(cache: pathlib.Path, repository: str) -> pathlib.Path:
    return cache / "eligibility-git" / (repository.lower().replace("/", "__") + ".git")


def bare_git(bare: pathlib.Path, args: list[str], timeout: int = 600) -> subprocess.CompletedProcess[str]:
    return command(["git", "--git-dir", str(bare), *args], timeout=timeout)


def changed_paths(bare: pathlib.Path, base: str, head: str, *, source_only: bool = False) -> list[str]:
    output = require(bare_git(bare, ["diff", "--name-only", base, head]), "listing changed files")
    paths = [line for line in output.splitlines() if line]
    if source_only:
        paths = [path for path in paths if path.endswith(SOURCE_SUFFIXES)]
    return paths


def all_paths(bare: pathlib.Path, revision: str) -> list[str]:
    output = require(bare_git(bare, ["ls-tree", "-r", "--name-only", revision]), "listing repository files")
    return [line for line in output.splitlines() if line]


def source_roots(paths: list[str]) -> list[str]:
    visible = [
        path for path in paths
        if not any(part.startswith(".") or part in SKIP_SEGMENTS for part in path.split("/"))
    ]
    non_source_holders: set[str] = set()
    source_files = []
    for path in visible:
        is_source = path.endswith(SOURCE_SUFFIXES)
        pieces = path.split("/")[:-1]
        ancestors = ["/".join(pieces[:n]) for n in range(len(pieces), -1, -1)]
        if is_source:
            source_files.append((path, ancestors))
        else:
            non_source_holders.update(ancestors)
    merged: set[str] = set()
    for _path, ancestors in source_files:
        unblocked = []
        for parent in ancestors:
            if parent in non_source_holders:
                break
            unblocked.append(parent)
        merged.add(unblocked[-1] if unblocked else ancestors[0])
    roots = []
    for path in sorted(merged):
        parts = path.split("/") if path else []
        parents = ["/".join(parts[:n]) for n in range(len(parts) - 1, -1, -1)]
        if not any(parent in merged for parent in parents):
            roots.append(path or ".")
    return roots


def export_tree(bare: pathlib.Path, revision: str, target: pathlib.Path) -> None:
    shutil.rmtree(target, ignore_errors=True)
    target.mkdir(parents=True)
    archive_path = target.parent / f".{target.name}.tar"
    archive_path.unlink(missing_ok=True)
    archived = git_retry(
        ["--git-dir", str(bare), "archive", "--format=tar", revision, "-o", str(archive_path)],
        timeout=600,
    )
    require(archived, f"exporting commit {revision}")
    require(command(["tar", "-xf", str(archive_path), "-C", str(target)]), "extracting commit tree")
    archive_path.unlink(missing_ok=True)


def prepare_worktree(bare: pathlib.Path, working: pathlib.Path, base: str, head: str) -> None:
    bare_git(bare, ["worktree", "remove", "--force", str(working)])
    shutil.rmtree(working, ignore_errors=True)
    require(bare_git(bare, ["worktree", "prune"]), "pruning old worktree")
    require(bare_git(bare, ["worktree", "add", "--detach", str(working), base]), "checking out study base")
    base_branch = f"study-base-{working.name}"
    require(command(["git", "-C", str(working), "checkout", "--force", "-B", base_branch, base]), "creating study base branch")
    change_branch = f"change-{working.name}"
    require(command(["git", "-C", str(working), "checkout", "--force", "-B", change_branch, head]), "checking out change")
    require(command(["git", "-C", str(working), "reset", "--hard", head]), "resetting study tree")
    require(command(["git", "-C", str(working), "clean", "-ffdx"]), "cleaning study tree")
    (working / "klin.json").write_text(json.dumps({"inventory": {}, "lockfile": {}}) + "\n")


def diff_line_count(bare: pathlib.Path, base: str, head: str, paths: list[str]) -> int:
    if not paths:
        return 0
    output = require(bare_git(bare, ["diff", "--numstat", base, head, "--", *paths]), "counting changed source lines")
    return sum(
        int(part)
        for row in output.splitlines()
        for part in row.split("\t", 2)[:2]
        if part.isdigit()
    )


def relevant(phenotype: str, language: str, files: list[str]) -> list[str]:
    paths = set(files)
    if phenotype in {"shipped-complexity", "shipped-escapes", "shipped-stubs", "shipped-test-skip"}:
        return [path for path in files if path.endswith(LANGUAGES[language])]
    if phenotype in {"shipped-dead-symbols", "shipped-reachability"}:
        return [path for path in files if language in ("Rust", "TypeScript") and path.endswith(LANGUAGES[language])]
    if phenotype == "shipped-public-api":
        suffixes = LANGUAGES[language] + (("Cargo.toml",) if language == "Rust" else ("package.json",))
        return [path for path in files if language in ("Rust", "TypeScript") and path.endswith(suffixes)]
    if phenotype == "shipped-test-deletion":
        return [path for path in files if path.endswith(LANGUAGES[language])]
    if phenotype == "shipped-lockfile":
        return [path for path in files if path.endswith(("Cargo.toml", "Cargo.lock", "package.json", "package-lock.json"))]
    if phenotype == "shipped-module-cycle":
        return [path for path in files if path.endswith(LANGUAGES[language]) and language in ("Rust", "TypeScript")]
    if phenotype in ASSERT_CANDIDATES:
        return [path for path in files if path.endswith((".rs", ".ts", ".tsx", ".mts", ".cts"))]
    if phenotype in SHAPE_CANDIDATES:
        return [path for path in files if path.endswith((".py", ".pyi", ".rs", ".ts", ".tsx", ".mts", ".cts"))]
    if phenotype in RELATION_CANDIDATES:
        return [path for path in files if path.endswith((".rs", ".ts", ".tsx", ".mts", ".cts"))]
    if phenotype in {"python-uv-lock", "new-direct-dependency"}:
        suffixes = (
            "pyproject.toml", "uv.lock", "Cargo.toml", "package.json", "requirements.txt",
            "requirements-dev.txt", "setup.cfg", "setup.py", "Pipfile",
        )
        if phenotype == "python-uv-lock":
            suffixes = ("pyproject.toml", "uv.lock")
        return [path for path in files if path.endswith(suffixes)]
    if phenotype == "cargo-lock-entry-shape":
        return [path for path in files if path.endswith(("Cargo.toml", "Cargo.lock"))]
    if phenotype in {"ruff-python-injection", "ruff-python-swallowed"}:
        return [path for path in files if path.endswith((".py", ".pyi"))]
    return sorted(paths)


def lockfile_pair_scope(
    bare: pathlib.Path, base: str, head: str, language: str, changed: list[str],
) -> tuple[list[str], bool, bool]:
    formats = {
        "Rust": {"Cargo.toml": ("Cargo.lock",)},
        "TypeScript": {"package.json": ("package-lock.json", "pnpm-lock.yaml", "yarn.lock")},
    }.get(language, {})
    if not formats:
        return [], False, False

    before_paths = set(all_paths(bare, base))
    after_paths = set(all_paths(bare, head))
    changed_set = set(changed)

    def nearest(manifest: str, paths: set[str], locks: tuple[str, ...]) -> str | None:
        directory = pathlib.PurePosixPath(manifest).parent
        while True:
            for name in locks:
                candidate = name if str(directory) == "." else f"{directory}/{name}"
                if candidate in paths:
                    return candidate
            if str(directory) == ".":
                return None
            directory = directory.parent

    before_manifests = {
        path for path in before_paths if pathlib.PurePosixPath(path).name in formats
    }
    after_manifests = {
        path for path in after_paths if pathlib.PurePosixPath(path).name in formats
    }
    selected: set[str] = set()
    eligible = False
    deleted = False

    for manifest in after_manifests:
        locks = formats[pathlib.PurePosixPath(manifest).name]
        now_lock = nearest(manifest, after_paths, locks)
        before_lock = nearest(manifest, before_paths, locks) if manifest in before_manifests else None
        touched = manifest in changed_set or now_lock in changed_set or before_lock in changed_set
        if touched and (now_lock is not None or before_lock is not None):
            eligible = True
            selected.add(manifest)

    for manifest in before_manifests - after_manifests:
        locks = formats[pathlib.PurePosixPath(manifest).name]
        old_lock = nearest(manifest, before_paths, locks)
        if manifest in changed_set or old_lock in changed_set:
            if old_lock is not None:
                eligible = True
                deleted = True

    return sorted(selected), eligible, deleted


def direct_dependency_manifests(paths: list[str]) -> list[str]:
    selected = []
    for path in paths:
        name = pathlib.PurePosixPath(path).name
        if name in {"Cargo.toml", "package.json", "pyproject.toml", "setup.cfg", "setup.py", "Pipfile"}:
            selected.append(path)
        elif (
            re.fullmatch(r"requirements.*\.txt", name)
            or (pathlib.PurePosixPath(path).parent.name == "requirements" and name.endswith((".txt", ".in")))
        ):
            selected.append(path)
    return sorted(selected)


def git_file(bare: pathlib.Path, revision: str, path: str) -> str | None:
    result = bare_git(bare, ["show", f"{revision}:{path}"])
    return result.stdout if result.returncode == 0 else None


def clean_evidence(text: str | None, *, limit: int = 120) -> str | None:
    if text is None:
        return None
    rows = text.splitlines()
    redacted = []
    secret = re.compile(r"(?i)(password|passwd|secret|token|api[_-]?key|private[_-]?key)(\s*[=:]\s*)([^\s,;]+)")
    for line in rows:
        redacted.append(secret.sub(r"\1\2[redacted]", line))
    return "\n".join(redacted[:limit]) + ("\n…" if len(redacted) > limit else "")


def blind_observation(text: str, phenotype: dict[str, str]) -> str:
    redacted = text
    hidden_terms = (
        phenotype["id"], phenotype["family"], phenotype["source_ticket"],
        phenotype["detector_version"],
        "all-checks-removed", "assertion-removed", "disabled", "expected-changed",
        "new-test-unchecked", "family-bypass", "registration-bypass",
        "wrapper-bypass", "component-cycle", "ellipsis-body", "throw-body",
        "empty-handler", "default-handler", "broad-handler", "log-handler",
        "mock-name", "constant-return", "cargo-lock-entry-shape",
        "new-direct-dependency", "python-uv-lock", "ruff-python-injection",
        "ruff-python-swallowed", "complexity", "public-api", "reachability",
        "layering", "lockfile", "escape", "stub",
    )
    for token in hidden_terms:
        if token:
            redacted = re.sub(re.escape(token), "[finding family omitted]", redacted, flags=re.IGNORECASE)
    redacted = re.sub(r"(?i)\b(?:S102|S307|S602|S604|S605|S608|BLE001|S110|S112)\b", "[rule omitted]", redacted)
    redacted = re.sub(r"(?i)#\d+", "[ticket omitted]", redacted)
    redacted = AGENT_WORDS.sub("[authoring attribution omitted]", redacted)
    return clean_evidence(redacted, limit=8) or "The changed code at this site was flagged for review."


def blind_task_context(sample: dict[str, str]) -> str:
    text = f"{sample.get('title', '')}\n\n{sample.get('body', '')}".strip()
    text = re.sub(r"https?://\S+", "[external reference omitted]", text)
    for identifier in (sample.get("repository", ""), sample.get("author_login", ""), sample.get("head_branch", "")):
        if identifier:
            text = re.sub(re.escape(identifier), "[project metadata omitted]", text, flags=re.IGNORECASE)
    text = AGENT_WORDS.sub("[authoring attribution omitted]", text)
    text = re.sub(r"@[A-Za-z0-9_-]+", "[user omitted]", text)
    return clean_evidence(text, limit=60) or "Review the submitted change against the surrounding code."


def packet_context(bare: pathlib.Path, base: str, head: str, site: dict[str, object] | None) -> list[dict[str, object]]:
    if not site:
        return []
    path = str(site.get("path", ""))
    if not path:
        return []
    line = int(site.get("start_line") or 1)
    before, after = git_file(bare, base, path), git_file(bare, head, path)
    def excerpt(value: str | None) -> str | None:
        if value is None:
            return None
        lines = value.splitlines()
        start, end = max(0, line - 16), min(len(lines), line + 15)
        return clean_evidence("\n".join(lines[start:end]))
    return [{"path": path, "before": excerpt(before), "after": excerpt(after), "context": None}]


def parse_cli_findings(report: dict[str, object], gate_name: str) -> list[dict[str, object]]:
    findings = report.get("findings", [])
    if not isinstance(findings, list):
        return []
    return [item for item in findings if isinstance(item, dict) and item.get("gate") == gate_name]


def gate_record(report_value: dict[str, object] | None, name: str) -> dict[str, object] | None:
    if report_value is None:
        return None
    gates = report_value.get("gates")
    if not isinstance(gates, list):
        return None
    return next(
        (item for item in gates if isinstance(item, dict) and item.get("name") == name),
        None,
    )


def scoped_units(phenotype: str, report_value: dict[str, object] | None, fallback: int) -> int:
    gate_name = GATE_BY_PHENOTYPE.get(phenotype)
    gate = gate_record(report_value, gate_name) if gate_name else None
    if not gate:
        return fallback
    if phenotype == "shipped-complexity":
        return fallback
    if phenotype in {"shipped-dead-symbols", "shipped-reachability"}:
        after = (gate.get("names") or {}).get("after") or {}
        key = "declarations" if phenotype == "shipped-dead-symbols" else "files"
        return int(after.get(key, fallback))
    if phenotype == "shipped-public-api":
        return int((gate.get("surface") or {}).get("items", fallback))
    if phenotype == "shipped-module-cycle":
        return int((gate.get("graph") or {}).get("dependencies", fallback))
    return int((gate.get("coverage") or {}).get("found", fallback))


def proto_finding(phenotype: str, raw: list[str]) -> list[dict[str, object]]:
    found = []
    if phenotype in ASSERT_CANDIDATES:
        candidates = ASSERT_CANDIDATES[phenotype]
        for row in raw:
            fields = row.split("\t", 6)
            if len(fields) >= 6 and fields[0] in candidates and "excluded" not in fields[4].split(","):
                if phenotype == "test-expected-mirrors-production" and fields[1] != "mirrors-production":
                    continue
                found.append({"finding_id": ":".join(fields[:4]), "path": fields[2], "line": fields[3], "text": fields[-1]})
    elif phenotype in SHAPE_CANDIDATES:
        candidate = SHAPE_CANDIDATES[phenotype]
        for row in raw:
            fields = row.split("\t", 5)
            if len(fields) >= 5 and fields[0] == candidate and not fields[4].strip("-"):
                found.append({"finding_id": ":".join(fields[:4]), "path": fields[2], "line": fields[3], "text": fields[-1]})
    elif phenotype in RELATION_CANDIDATES:
        candidate = RELATION_CANDIDATES[phenotype]
        for row in raw:
            fields = row.split("\t", 3)
            if len(fields) >= 3 and fields[0] == candidate and fields[1] == "finding":
                location = fields[2]
                path, _, line = location.rpartition(":")
                found.append({"finding_id": f"{candidate}:{location}:{fields[-1]}", "path": path, "line": line, "text": fields[-1]})
    return found


def ruff_findings(root: pathlib.Path, files: list[str], phenotype: str, cache: pathlib.Path) -> tuple[list[dict[str, object]], str, list[str], int]:
    if not files:
        return [], "complete", [], 0
    rules = "S102,S307,S602,S604,S605,S608" if phenotype == "ruff-python-injection" else "BLE001,S110,S112"
    env = {**os.environ, "UV_CACHE_DIR": str(cache / "uv-cache"), "UV_TOOL_DIR": str(cache / "uv-tools")}
    started = time.monotonic()
    result = command(
        ["uv", "tool", "run", "--from", "ruff==0.16.10", "ruff", "check", "--output-format", "json",
         "--select", rules, "--isolated", "--no-cache", "--ignore-noqa", *files],
        cwd=root, env=env, timeout=600,
    )
    elapsed = round((time.monotonic() - started) * 1000, 3)
    if result.returncode not in (0, 1):
        return [], "tool-error", [result.stderr.strip() or f"Ruff exited {result.returncode}"], elapsed
    try:
        diagnostics = json.loads(result.stdout)
    except json.JSONDecodeError:
        return [], "tool-error", ["Ruff returned invalid JSON"], elapsed
    invalid = [str(item.get("filename", "")) for item in diagnostics if item.get("code") == "invalid-syntax"]
    findings = []
    for item in diagnostics:
        code = str(item.get("code", ""))
        if code not in rules.split(","):
            continue
        filename = str(item.get("filename", ""))
        filename = pathlib.Path(filename).relative_to(root).as_posix() if pathlib.Path(filename).is_absolute() else filename
        location = item.get("location") or {}
        message = str(item.get("message", ""))
        findings.append({
            "finding_id": f"{code}:{filename}:{location.get('row', '')}:{location.get('column', '')}:{message}",
            "path": filename, "line": location.get("row"), "text": f"{code}: {message}",
        })
    state = "invalid-syntax" if invalid else "complete"
    holes = [f"Ruff could not parse {path}" for path in invalid]
    return findings, state, holes, elapsed


def write_json_tsv(path: pathlib.Path, fields: tuple[str, ...], rows: list[dict[str, object]]) -> None:
    serial = []
    for row in rows:
        serial.append({
            key: json.dumps(value, ensure_ascii=False, sort_keys=True) if isinstance(value, (dict, list)) else value
            for key, value in row.items()
        })
    write_tsv(path, fields, serial)


def replay(cache: pathlib.Path) -> None:
    sample_path = HERE / "natural-sample.tsv"
    if not sample_path.is_file():
        raise StudyError("natural-sample.tsv is missing; run materialize first")
    tools = build_tools(cache)
    source = tools["source"]
    klin = tools["klin"]
    bins = tools["bins"]
    phenotypes = list(csv.DictReader((HERE / "phenotypes.tsv").open(encoding="utf-8"), delimiter="\t"))
    samples = list(csv.DictReader(sample_path.open(encoding="utf-8"), delimiter="\t"))
    output = cache / "study-v2" / "replay"
    trees = output / "trees"
    runs = output / "runs"
    trees.mkdir(parents=True, exist_ok=True)
    runs.mkdir(parents=True, exist_ok=True)
    measurement_rows: list[dict[str, object]] = []
    finding_rows: list[dict[str, object]] = []

    for index, sample in enumerate(samples, 1):
        change_id = sample["change_id"]
        repository = sample["repository"]
        base, head = sample["base"], sample["head"]
        language = sample["language"]
        bare = bare_repository(cache, repository)
        if not bare.exists():
            raise StudyError(f"missing fetched repository for {repository}; use the same --cache as materialize")
        base_tree, head_tree = trees / f"{change_id}-base", trees / f"{change_id}-head"
        export_tree(bare, base, base_tree)
        export_tree(bare, head, head_tree)
        files = changed_paths(bare, base, head)
        lockfile_manifests, lockfile_eligible, lockfile_deleted = lockfile_pair_scope(
            bare, base, head, language, files,
        )
        code_files = [path for path in files if path.endswith(LANGUAGES[language])]
        proto_files = [
            path for path in files
            if (head_tree / path).is_file()
            and path.endswith((".rs", ".py", ".pyi", ".ts", ".tsx", ".mts", ".cts"))
        ]
        all_head = all_paths(bare, head)
        dependency_manifest_paths = direct_dependency_manifests(files)
        roots = source_roots(all_head)
        # Worktrees share the cached bare repository with v1, so their Git branch names
        # must be distinct even though their output directories are already versioned.
        working = output / "worktrees" / f"study-v2-{change_id}"
        working.parent.mkdir(parents=True, exist_ok=True)
        prepare_worktree(bare, working, base, head)
        # `inventory` needs an explicit scoped policy in klin 0.4.2. Excluding
        # Git metadata preserves its effective whole-repository test scope;
        # `--changed` below narrows the replay to the change under study.
        config: dict[str, object] = {"inventory": {"except": ".git"}}
        config["lockfile"] = {"in": lockfile_manifests} if lockfile_manifests else False
        if roots:
            config["layering"] = {
                "acyclic": True,
                "layers": {"study-all": {"in": roots, "can_use": None}},
            }
        (working / "klin.json").write_text(json.dumps(config, indent=2) + "\n")
        gate_env = {key: value for key, value in os.environ.items() if not key.startswith("KLIN_")}
        gate_env.pop("GITHUB_EVENT_PATH", None)
        gate_env["GITHUB_BASE_REF"] = f"study-base-{working.name}"
        started = time.monotonic()
        gate_result = command([str(klin), "gate", "--strict", "--json"], cwd=working, env=gate_env, timeout=900)
        gate_ms = round((time.monotonic() - started) * 1000, 3)
        gate_json: dict[str, object] | None = None
        try:
            gate_json = json.loads(gate_result.stdout)
        except json.JSONDecodeError:
            pass
        (runs / f"{change_id}.json").write_text(json.dumps({
            "change_id": change_id, "exit": gate_result.returncode, "runtime_ms": gate_ms,
            "report": gate_json, "stdout": None if gate_json is not None else gate_result.stdout,
            "stderr": gate_result.stderr,
        }, indent=2) + "\n")

        scope_args = [str(klin), "gate", "--changed", "--json"]
        for gate_name in (
            "complexity", "escapes", "stubs", "inventory", "dead-symbols", "reachability",
            "lockfile", "layering", "public-api",
        ):
            scope_args.extend(["--gate", gate_name])
        scope_result = command(scope_args, cwd=working, env=gate_env, timeout=900)
        try:
            scope_json = json.loads(scope_result.stdout)
        except json.JSONDecodeError:
            scope_json = None
        (runs / f"{change_id}-scope.json").write_text(json.dumps({
            "change_id": change_id, "exit": scope_result.returncode,
            "lockfile_pair_manifests": lockfile_manifests,
            "report": scope_json, "stdout": None if scope_json is not None else scope_result.stdout,
            "stderr": scope_result.stderr,
        }, indent=2) + "\n")

        lockfile_text = None
        lockfile_runtime = 0.0
        if lockfile_manifests:
            lockfile_started = time.monotonic()
            lockfile_text = command(
                [str(klin), "gate", "--changed", "--gate", "lockfile"],
                cwd=working, env=gate_env, timeout=900,
            )
            lockfile_runtime = round((time.monotonic() - lockfile_started) * 1000, 3)
        lockfile_count_match = re.search(
            r"\b(\d+) dependenc(?:y|ies) in (\d+) manifest\(s\)",
            lockfile_text.stdout if lockfile_text is not None else "",
        )
        lockfile_gate = gate_record(scope_json, "lockfile")
        lockfile_coverage = (lockfile_gate or {}).get("coverage") or {}
        dependency_count = int(lockfile_count_match.group(1)) if lockfile_count_match else None
        judged_manifest_count = int(lockfile_count_match.group(2)) if lockfile_count_match else None
        lockfile_census_complete = (
            not lockfile_eligible
            or (
                bool(lockfile_manifests)
                and not lockfile_deleted
                and dependency_count is not None
                and judged_manifest_count == len(lockfile_manifests)
                and int(lockfile_coverage.get("measured", 0) or 0) == len(lockfile_manifests)
                and int(lockfile_coverage.get("unreadable", 0) or 0) == 0
                and int((lockfile_gate or {}).get("notes", 0) or 0) == 0
                and (lockfile_gate or {}).get("status") not in {"unknown", "error"}
            )
        )
        (runs / f"{change_id}-lockfile-census.json").write_text(json.dumps({
            "change_id": change_id,
            "eligible_pair": lockfile_eligible,
            "eligible_manifests": lockfile_manifests,
            "deleted_manifest_pair": lockfile_deleted,
            "eligible_units": dependency_count,
            "judged_manifests": judged_manifest_count,
            "unit_census_complete": lockfile_census_complete,
            "runtime_ms": lockfile_runtime,
            "exit": lockfile_text.returncode if lockfile_text is not None else None,
            "stdout": lockfile_text.stdout if lockfile_text is not None else None,
            "stderr": lockfile_text.stderr if lockfile_text is not None else None,
        }, indent=2) + "\n")

        proto = {"asserts": [], "relations": [], "shapes": []}
        if proto_files:
            kinds = ["asserts"] if language in ("Rust", "TypeScript") else ["shapes"]
            for kind in kinds:
                tool = bins[kind]
                proto_started = time.monotonic()
                result = command([str(tool), "new", str(base_tree), str(head_tree), *proto_files], timeout=900)
                proto[kind + "_runtime_ms"] = round((time.monotonic() - proto_started) * 1000, 3)
                if result.returncode:
                    proto[kind + "_error"] = result.stderr.strip() or f"exit {result.returncode}"
                else:
                    proto[kind] = [line for line in result.stdout.splitlines() if line]
        if language in ("Rust", "TypeScript") and proto_files:
            proto_started = time.monotonic()
            relation = command([str(bins["relations"]), "new", str(base_tree), str(head_tree)], timeout=900)
            proto["relations_runtime_ms"] = round((time.monotonic() - proto_started) * 1000, 3)
            if relation.returncode:
                proto["relations_error"] = relation.stderr.strip() or f"exit {relation.returncode}"
            else:
                proto["relations"] = [line for line in relation.stdout.splitlines() if line]
        dep = {"added": [], "lockshape": [], "added_error": None, "manifest_paths": dependency_manifest_paths}
        dependency_manifest_touched = bool(dependency_manifest_paths)
        if dependency_manifest_touched:
            dep_script = source / "docs/dependency-evidence-2026-10-02/prototype/deps.py"
            dep_started = time.monotonic()
            result = command([sys.executable, str(dep_script), "added", str(base_tree), str(head_tree)], timeout=900)
            dep["added_runtime_ms"] = round((time.monotonic() - dep_started) * 1000, 3)
            if result.returncode:
                dep["added_error"] = result.stderr.strip() or f"exit {result.returncode}"
            else:
                dep["added"] = [line for line in result.stdout.splitlines() if line]
        if any(path.endswith(("Cargo.lock", "package-lock.json")) for path in files):
            dep_tool = source / "docs/dependency-evidence-2026-10-02/prototype/deps.py"
            dep_started = time.monotonic()
            old = command([sys.executable, str(dep_tool), "lockshape", str(base_tree)], timeout=900)
            new = command([sys.executable, str(dep_tool), "lockshape", str(head_tree)], timeout=900)
            dep["lockshape_runtime_ms"] = round((time.monotonic() - dep_started) * 1000, 3)
            if old.returncode or new.returncode:
                dep["lockshape_error"] = old.stderr.strip() or new.stderr.strip() or "lockshape failed"
            else:
                old_keys = {"\t".join(line.split("\t", 2)[:2]) for line in old.stdout.splitlines() if line}
                dep["lockshape"] = [line for line in new.stdout.splitlines() if line and "\t" in line and "\t".join(line.split("\t", 2)[:2]) not in old_keys]
        (runs / f"{change_id}-prototypes.json").write_text(json.dumps(proto, indent=2) + "\n")
        (runs / f"{change_id}-dependencies.json").write_text(json.dumps(dep, indent=2) + "\n")

        changed_complexity_count = 0
        if code_files:
            result = command([str(klin), "gate", "--changed", "--gate", "complexity"], cwd=working, env=gate_env, timeout=900)
            match = re.search(r"(\d+) function\(s\) judged", result.stdout)
            changed_complexity_count = int(match.group(1)) if match else 0

        for phenotype in phenotypes:
            pid = phenotype["id"]
            languages = phenotype["languages"].split("|")
            relevant_files = relevant(pid, language, files)
            semantic = language in languages and bool(relevant_files)
            lockfile_census_path = runs / f"{change_id}-lockfile-census.json"
            try:
                lockfile_census = json.loads(lockfile_census_path.read_text(encoding="utf-8"))
            except (OSError, json.JSONDecodeError):
                lockfile_census = {}
            if pid == "shipped-lockfile":
                semantic = language in languages and lockfile_census.get("eligible_pair") is True
            elif pid == "new-direct-dependency":
                semantic = language in languages and bool(dependency_manifest_paths)
            if pid == "shipped-test-deletion":
                inventory = gate_record(scope_json, "inventory")
                coverage = (inventory or {}).get("coverage") or {}
                candidate_files = int(coverage.get("found", 0))
                before_tests = [
                    path for path in relevant_files
                    if (base_tree / path).is_file()
                    and (
                        any(part in {"test", "tests", "spec", "__tests__"} for part in pathlib.PurePosixPath(path).parts[:-1])
                        or ".test." in path
                        or path.endswith(("_test.rs", "_test.py", "_test.ts"))
                    )
                ]
                semantic = language in languages and (candidate_files > 0 or bool(before_tests))
            if pid == "shipped-module-cycle" and not roots:
                semantic = language in languages and bool(relevant_files)
            if pid in {"ruff-python-injection", "ruff-python-swallowed"}:
                relevant_files = [path for path in relevant_files if (head_tree / path).is_file()]
                semantic = language in languages and bool(relevant_files)

            row_id = f"{change_id}:{pid}"
            state, holes, findings, runtime, measured = "unsupported", [], [], 0.0, 0
            units = 0
            eligibility_unit_method = "none"
            denominator_hole = None
            if semantic:
                if pid == "shipped-complexity":
                    units = changed_complexity_count
                    eligibility_unit_method = "changed functions parsed by klin complexity"
                elif pid in {"ruff-python-injection", "ruff-python-swallowed"}:
                    units = len(relevant_files)
                    eligibility_unit_method = "changed Python files passed to the registered Ruff family"
                elif pid == "shipped-test-deletion":
                    inventory = gate_record(scope_json, "inventory")
                    coverage = (inventory or {}).get("coverage") or {}
                    units = int(coverage.get("found", 0))
                    eligibility_unit_method = "before-tree test candidate files in changed scope"
                    denominator_hole = "the frozen inventory coverage counts candidate files, not individual test identities"
                    if units == 0:
                        units = len(before_tests)
                        denominator_hole = "inventory candidate coverage was unavailable; eligible before-tree test files are a path-based proxy"
                elif pid == "shipped-lockfile":
                    dependency_count = lockfile_census.get("eligible_units")
                    units = int(dependency_count) if isinstance(dependency_count, int) else 0
                    eligibility_unit_method = "direct dependencies in changed supported manifest-lock pairs counted by the frozen CLI"
                    if lockfile_census.get("unit_census_complete") is not True:
                        denominator_hole = "the frozen lockfile CLI did not expose a complete eligible-unit census for dependency sites"
                elif pid == "new-direct-dependency":
                    units = 0
                    eligibility_unit_method = "direct dependency names in changed supported manifests"
                    denominator_hole = "the frozen dependency prototype does not expose a complete changed-manifest eligible-unit census"
                else:
                    units = diff_line_count(bare, base, head, relevant_files)
                    units = scoped_units(pid, scope_json, units)
                    eligibility_unit_method = "changed source lines in registered file scope"
                    if pid in ASSERT_CANDIDATES or pid in SHAPE_CANDIDATES or pid in RELATION_CANDIDATES:
                        denominator_hole = "the frozen prototype does not expose a complete eligible-unit census; changed source lines are a proxy"
                    elif pid == "shipped-test-skip":
                        denominator_hole = "changed test candidates and skip-capable sites are not separately counted; changed source lines are a proxy"
                    elif pid == "python-uv-lock":
                        denominator_hole = "eligible direct Python dependency entries are not separately enumerated by the frozen lock predicate"
                    elif pid == "cargo-lock-entry-shape":
                        denominator_hole = "eligible registry lock entries are not separately enumerated by the frozen shape predicate"
                if pid not in {"new-direct-dependency", "shipped-lockfile"} and units == 0:
                    if pid == "shipped-complexity":
                        gate = gate_record(scope_json, "complexity")
                        coverage = (gate or {}).get("coverage") or {}
                        if coverage.get("not_measured", 0) or coverage.get("unreadable", 0):
                            denominator_hole = "changed complexity files are unreadable or not measured; their eligible function count is unknown"
                        else:
                            semantic = False
                    elif denominator_hole is None:
                        semantic = False
            if semantic:
                state, runtime, measured = "complete", 0.0, units
                gate_name = GATE_BY_PHENOTYPE.get(pid)
                if pid == "shipped-test-deletion":
                    gate_name = "inventory"
                if gate_name:
                    if gate_json is None:
                        state, measured = "tool-error", 0
                        holes.append("klin did not return a JSON report")
                    else:
                        gate_rows = gate_json.get("gates") or []
                        gate_row = next((item for item in gate_rows if isinstance(item, dict) and item.get("name") == gate_name), None)
                        if gate_row is None:
                            state, measured = "unsupported", 0
                            holes.append(f"frozen klin did not run {gate_name}")
                        else:
                            findings = [
                                {"finding_id": item.get("id", ""), "path": item.get("file"), "line": item.get("line"), "text": item.get("text", "")}
                                for item in parse_cli_findings(gate_json, gate_name)
                                if item.get("outcome") in {"new", "worsened"}
                            ]
                            unresolved = [
                                item for item in parse_cli_findings(gate_json, gate_name)
                                if item.get("outcome") in {"unparsed", "unresolved", "unknown"}
                            ]
                            if unresolved:
                                state, measured = "partial", 0
                                holes.extend(
                                    f"{item.get('file', '')}:{item.get('line', '')}: {item.get('outcome')} {item.get('text', '')}"
                                    for item in unresolved
                                )
                            coverage = gate_row.get("coverage") or {}
                            if gate_row.get("status") in {"unknown", "error"}:
                                state, measured = "tool-error", 0
                                holes.append(f"{gate_name} status is {gate_row.get('status')}")
                            elif coverage.get("not_measured", 0) or coverage.get("unreadable", 0):
                                state, measured = "partial", 0
                                holes.append(f"{gate_name} coverage has {coverage.get('not_measured', 0)} unmeasured and {coverage.get('unreadable', 0)} unreadable files")
                            surface = gate_row.get("surface") or {}
                            if surface.get("holes", 0) or surface.get("opaque", 0):
                                state, measured = "partial", 0
                                holes.append(f"{gate_name} public surface has {surface.get('holes', 0)} holes and {surface.get('opaque', 0)} opaque items")
                            for note in gate_json.get("notes", []):
                                rendered = json.dumps(note, ensure_ascii=False) if isinstance(note, dict) else str(note)
                                if gate_name.lower() in rendered.lower() and any(word in rendered.lower() for word in ("unresolved", "unparsed", "incomplete", "unknown")):
                                    state, measured = "partial", 0
                                    holes.append(rendered)
                            runtime = float(gate_row.get("ms", 0))
                            if pid == "shipped-test-deletion" and state == "unsupported":
                                holes.append("inventory was not enabled by the frozen study configuration")
                elif pid in ASSERT_CANDIDATES:
                    result = proto_finding(pid, proto["asserts"])
                    findings = result
                    runtime = float(proto.get("asserts_runtime_ms", 0))
                    if "asserts_error" in proto:
                        state, measured, holes = "tool-error", 0, [str(proto["asserts_error"])]
                elif pid in SHAPE_CANDIDATES:
                    findings = proto_finding(pid, proto["shapes"])
                    runtime = float(proto.get("shapes_runtime_ms", 0))
                    if "shapes_error" in proto:
                        state, measured, holes = "tool-error", 0, [str(proto["shapes_error"])]
                elif pid in RELATION_CANDIDATES:
                    findings = proto_finding(pid, proto["relations"])
                    runtime = float(proto.get("relations_runtime_ms", 0))
                    if "relations_error" in proto:
                        state, measured, holes = "tool-error", 0, [str(proto["relations_error"])]
                    elif pid == "design-component-cycle" and language == "TypeScript" and any("unknown" in line for line in proto["relations"]):
                        state, measured = "local-resolution-incomplete", 0
                        holes.append("the frozen ModuleGraph prototype reports unresolved local dependency evidence")
                elif pid == "new-direct-dependency":
                    runtime = float(dep.get("added_runtime_ms", 0))
                    for line in dep["added"]:
                        fields = line.split("\t")
                        if len(fields) >= 3:
                            findings.append({"finding_id": f"{fields[0]}:{fields[1]}", "path": fields[2], "line": None, "text": f"new direct dependency {fields[0]}:{fields[1]}"})
                    if dep["added_error"]:
                        state, measured, holes = "tool-error", 0, [str(dep["added_error"])]
                elif pid == "cargo-lock-entry-shape":
                    runtime = float(dep.get("lockshape_runtime_ms", 0))
                    findings = [
                        {"finding_id": ":".join(line.split("\t", 2)), "path": line.split("\t", 2)[0], "line": None, "text": line.split("\t", 2)[-1]}
                        for line in dep["lockshape"] if len(line.split("\t")) >= 3
                    ]
                    if dep.get("lockshape_error"):
                        state, measured, holes = "tool-error", 0, [str(dep["lockshape_error"])]
                elif pid == "python-uv-lock":
                    findings, state, holes, runtime = uv_lock_findings(base_tree, head_tree, relevant_files)
                    measured = units if state == "complete" else 0
                elif pid in {"ruff-python-injection", "ruff-python-swallowed"}:
                    findings, state, holes, runtime = ruff_findings(working, relevant_files, pid, cache)
                    measured = len(relevant_files) if state == "complete" else max(0, len(relevant_files) - len(holes))

            if semantic and pid == "shipped-module-cycle" and not roots:
                state, measured = "unsupported", 0
                holes.append("the frozen survey found no source root for layering")
            if semantic and denominator_hole:
                if state == "complete":
                    state = "partial"
                if denominator_hole not in holes:
                    holes.append(denominator_hole)
            population_id = sample["population"]
            measurement = {
                "row_id": row_id, "study_version": 2, "change_id": change_id,
                "population_id": population_id, "repository": repository, "base": base, "head": head,
                "language": language, "phenotype_id": pid, "semantic_eligible": semantic,
                "eligibility_count": units if semantic else 0, "measurement_state": state,
                "measured_count": measured, "runtime_ms": runtime,
                "measurement_basis": phenotype["measurement_basis"], "holes": holes,
                "values": {"finding_count": len(findings), "eligible_units": units if semantic else 0,
                           "eligibility_unit_method": eligibility_unit_method,
                           "tool_failure": state == "tool-error", "measurement_scope_files": relevant_files},
            }
            measurement_rows.append(measurement)
            for ordinal, finding in enumerate(findings, 1):
                finding_row_id = f"{row_id}:{ordinal}"
                source_id = "natural:" + finding_row_id
                finding_site = {
                    "path": str(finding.get("path") or ""),
                    "start_line": int(finding["line"]) if str(finding.get("line", "")).isdigit() else None,
                    "end_line": int(finding["line"]) if str(finding.get("line", "")).isdigit() else None,
                    "start_byte": None, "end_byte": None, "owner": None,
                }
                finding_rows.append({
                    "finding_row_id": finding_row_id, "study_version": 2, "measurement_row_id": row_id,
                    "source_finding_id": source_id, "finding_id": str(finding.get("finding_id", ordinal)),
                    "finding_site": finding_site, "evidence_packet_id": None,
                    "_change": sample, "_phenotype": pid, "_finding": finding,
                })
        print(f"replayed {index}/{len(samples)} {change_id}", flush=True)

    hard_results, hard_packet_rows = replay_fixture_hard_negatives(
        cache, klin, bins, source, phenotypes
    )
    write_json_tsv(HERE / "hard-negative-results.tsv", (
        "study_version", "case_set_id", "case_id", "phenotype_id",
        "measurement_state", "finding_ids", "result", "reason",
    ), hard_results)
    reconcile_measurements(measurement_rows, cache)
    write_json_tsv(HERE / "measurements.tsv", MEASUREMENT_FIELDS, measurement_rows)
    coordinator_path = HERE / "packet-coordinator.json"
    if coordinator_path.is_file():
        coordinator = json.loads(coordinator_path.read_text(encoding="utf-8"))
        salt = str(coordinator.get("salt", ""))
        if not re.fullmatch(r"[0-9a-f]{64}", salt):
            raise StudyError("packet coordinator salt is invalid")
        if coordinator.get("salt_sha256") != hashlib.sha256(bytes.fromhex(salt)).hexdigest():
            raise StudyError("packet coordinator salt digest does not match")
    else:
        salt = os.urandom(32).hex()
    packet_records = finding_rows + hard_packet_rows
    ordered_findings = sorted(
        packet_records,
        key=lambda row: (hashlib.sha256(f"357-label-v1:{salt}:{row['source_finding_id']}".encode()).hexdigest(), row["source_finding_id"]),
    )
    manifest_rows = []
    phenotype_registry = {row["id"]: row for row in phenotypes}
    packets_dir = HERE / "evidence-packets"
    shutil.rmtree(packets_dir, ignore_errors=True)
    packets_dir.mkdir()
    packet_ids: dict[str, str] = {}
    for ordinal, record in enumerate(ordered_findings, 1):
        packet_id = f"P{ordinal:05d}"
        source_id = str(record["source_finding_id"])
        packet_ids[source_id] = packet_id
        sample = record["_change"]
        finding = record["_finding"]
        site = record["finding_site"]
        is_natural = record.get("source_kind", "natural") == "natural"
        if is_natural:
            measurement_row = next(
                row for row in measurement_rows if row["row_id"] == record["measurement_row_id"]
            )
            measurement_holes = measurement_row["holes"]
            task_context = blind_task_context(sample)
            diff_context = packet_context(
                bare_repository(cache, str(sample["repository"])), str(sample["base"]), str(sample["head"]), site
            )
        else:
            measurement_holes = record.get("_holes", [])
            task_context = str(record.get("_task_context", "Review the submitted change against its base tree."))
            diff_context = record.get("_diff_context", [])
        packet = {
            "packet_id": packet_id,
            "source_finding_id": packet_id,
            "study_version": 2,
            "language": sample["language"],
            "task_context": task_context,
            "diff_context": diff_context,
            "observations": [blind_observation(
                str(finding.get("text", "candidate finding")),
                phenotype_registry.get(
                    str(record.get("_phenotype", record.get("phenotype_id", ""))),
                    {"id": "", "family": "", "source_ticket": "", "detector_version": ""},
                ),
            )],
            "measurement_holes": list(measurement_holes),
            "blindness_revealed": [],
            "unblind_requests": [],
        }
        (packets_dir / f"{packet_id}.json").write_text(json.dumps(packet, ensure_ascii=False, indent=2) + "\n")
        record["evidence_packet_id"] = packet_id
        manifest_rows.append({
            "source_finding_id": source_id, "packet_id": packet_id,
            "source_kind": record.get("source_kind", "natural"),
            "change_id": record.get("change_id", sample.get("change_id", "")),
            "case_set_id": record.get("case_set_id", ""),
            "case_id": record.get("case_id", ""),
            "phenotype_id": record.get("_phenotype", record.get("phenotype_id", "")),
            "finding_row_id": record.get("finding_row_id", ""),
        })
    for finding in finding_rows:
        finding.pop("_change", None)
        finding.pop("_phenotype", None)
        finding.pop("_finding", None)
    write_json_tsv(HERE / "findings.tsv", FINDING_FIELDS, finding_rows)
    write_tsv(HERE / "packet-manifest.tsv", (
        "source_finding_id", "packet_id", "source_kind", "change_id", "case_set_id",
        "case_id", "phenotype_id", "finding_row_id",
    ), manifest_rows)
    packet_meta = {
        "study_version": 2, "salt": salt,
        "salt_sha256": hashlib.sha256(bytes.fromhex(salt)).hexdigest(),
        "packet_count": len(packet_ids),
    }
    coordinator_path.write_text(json.dumps(packet_meta, indent=2) + "\n")
    report(cache)


def uv_lock_findings(before: pathlib.Path, after: pathlib.Path, changed: list[str]) -> tuple[list[dict[str, object]], str, list[str], float]:
    import tomllib
    started = time.monotonic()
    findings: list[dict[str, object]] = []
    holes: list[str] = []
    for path in sorted({str(pathlib.PurePosixPath(item).parent / "pyproject.toml") for item in changed if item.endswith(("uv.lock", "pyproject.toml"))}):
        manifest_path = after / path
        lock_path = after / (str(pathlib.PurePosixPath(path).parent / "uv.lock"))
        if not manifest_path.is_file() or not lock_path.is_file():
            missing = "pyproject.toml" if not manifest_path.is_file() else "uv.lock"
            holes.append(f"{path}: paired {missing} is absent from the after tree")
            continue
        try:
            project = tomllib.loads(manifest_path.read_text())
            lock = tomllib.loads(lock_path.read_text())
        except (OSError, tomllib.TOMLDecodeError) as error:
            holes.append(f"{path}: {error}")
            continue
        required: dict[str, set[str]] = {}
        project_data = project.get("project", {}) or {}
        specs = list(project_data.get("dependencies", []) or [])
        for group in (project_data.get("optional-dependencies", {}) or {}).values():
            specs.extend(group or [])
        for group in (project.get("dependency-groups", {}) or {}).values():
            specs.extend(item for item in group or [] if isinstance(item, str))
        for spec in specs:
            match = re.match(r"\s*([A-Za-z0-9][A-Za-z0-9._-]*)(?:\[[^]]+\])?\s*==\s*([A-Za-z0-9.!+_-]+)\s*$", str(spec))
            if match and "*" not in match.group(2):
                required.setdefault(re.sub(r"[-_.]+", "-", match.group(1)).lower(), set()).add(match.group(2))
        locked: dict[str, set[str]] = {}
        for item in lock.get("package", []) or []:
            source = item.get("source") or {}
            if source.get("editable") is not None or source.get("virtual") is not None or source.get("directory") is not None or source.get("path") is not None:
                continue
            key = re.sub(r"[-_.]+", "-", str(item.get("name", ""))).lower()
            locked.setdefault(key, set()).add(str(item.get("version", "")))
        for name, versions in sorted(required.items()):
            if not versions.intersection(locked.get(name, set())):
                findings.append({
                    "finding_id": f"{path}:{name}:{','.join(sorted(versions))}", "path": path,
                    "line": None, "text": f"uv.lock does not contain the exact manifest pin for {name}",
                })
    state = "partial" if holes else "complete"
    elapsed = round((time.monotonic() - started) * 1000, 3)
    return findings, state, holes, elapsed


def tree_paths(root: pathlib.Path) -> list[str]:
    return sorted(path.relative_to(root).as_posix() for path in root.rglob("*") if path.is_file() and ".git" not in path.parts)


def fixture_routes(case_set_id: str) -> list[tuple[str, pathlib.Path, pathlib.Path, str]]:
    root_by_set = {
        "test-integrity-corpus": HERE.parent / "test-integrity-2026-10-02" / "fixtures",
        "design-conformance-corpus": HERE.parent / "design-conformance-2026-10-02" / "fixtures",
        "unfinished-code-corpus": HERE.parent / "unfinished-code-2026-10-02" / "fixtures",
        "dependency-evidence-corpus": HERE.parent / "dependency-evidence-2026-10-02" / "fixtures",
    }
    if case_set_id.startswith("ruff-"):
        root = HERE.parent / "analyzer-recipes-2026-10-03" / "fixtures" / "recipes-py"
        routes_path = root / "routes.tsv"
        family = "injection" if case_set_id == "ruff-injection-corpus" else "swallowed"
        rows = csv.reader(routes_path.open(encoding="utf-8"), delimiter="\t")
        return [
            (route_id, root / "base", root / route_id, "Python")
            for route_id, route_family, category in rows
            if route_family == family and category == "negative"
        ]
    root = root_by_set.get(case_set_id)
    if root is None:
        return []
    found = []
    for family in sorted(root.iterdir()):
        if not family.is_dir() or not (family / "base").is_dir():
            continue
        language = "Rust" if family.name.endswith("-rs") or family.name == "lockshape-cargo" else (
            "Python" if family.name.endswith("-py") or family.name == "repair-py" else "TypeScript"
        )
        for route in sorted(family.iterdir()):
            if not route.is_dir() or route.name == "base":
                continue
            if route.name == "legit" or route.name.startswith(("legit-", "neg-")):
                found.append((f"{family.name}/{route.name}", family / "base", route, language))
    return found


def fixture_tree(
    cache: pathlib.Path, case_set_id: str, case_id: str,
    base_source: pathlib.Path, route: pathlib.Path,
) -> tuple[pathlib.Path, pathlib.Path, pathlib.Path, list[str], str, str]:
    slug = hashlib.sha256(f"{case_set_id}:{case_id}".encode()).hexdigest()[:16]
    root = cache / "study-v2" / "hard-negative" / slug
    shutil.rmtree(root, ignore_errors=True)
    root.mkdir(parents=True)
    working, before, after = root / "repo", root / "before", root / "after"
    shutil.copytree(base_source, working)
    if not (working / "klin.json").exists():
        (working / "klin.json").write_text("{}\n")

    identity = ["git", "-c", "user.name=klin-study", "-c", "user.email=study@example.invalid"]
    require(command(["git", "init", "-q", "-b", "main"], cwd=working), f"initializing hard-negative case {case_id}")
    require(command(["git", "add", "-A"], cwd=working), f"staging hard-negative base {case_id}")
    require(command(identity + ["commit", "-q", "-m", "frozen base"], cwd=working), f"committing hard-negative base {case_id}")
    base = require(command(["git", "rev-parse", "HEAD"], cwd=working), f"reading hard-negative base {case_id}").strip()
    shutil.copytree(working, before, ignore=shutil.ignore_patterns(".git"))
    require(command(["git", "checkout", "-q", "-b", "change"], cwd=working), f"creating hard-negative change branch {case_id}")

    shutil.copytree(route, working, dirs_exist_ok=True)
    family = route.parent
    deletions = family / f"{route.name}.delete"
    if deletions.is_file():
        for relative in deletions.read_text(encoding="utf-8").splitlines():
            relative = relative.strip()
            if not relative or relative.startswith("#"):
                continue
            target = working / relative
            if target.is_dir():
                shutil.rmtree(target)
            else:
                target.unlink(missing_ok=True)
    for relative in tree_paths(route):
        target = working / relative
        if target.is_file():
            os.utime(target, None)
    require(command(["git", "add", "-A"], cwd=working), f"staging hard-negative route {case_id}")
    require(command(identity + ["commit", "-q", "-m", "frozen route"], cwd=working), f"committing hard-negative route {case_id}")
    head = require(command(["git", "rev-parse", "HEAD"], cwd=working), f"reading hard-negative head {case_id}").strip()
    shutil.copytree(working, after, ignore=shutil.ignore_patterns(".git"))
    changed = require(command(["git", "diff", "--name-only", f"{base}...{head}"], cwd=working), f"listing hard-negative changes {case_id}").splitlines()

    config_path = working / "klin.json"
    try:
        config = json.loads(config_path.read_text(encoding="utf-8"))
    except (OSError, json.JSONDecodeError):
        config = {}
    if not isinstance(config, dict):
        config = {}
    config.setdefault("inventory", {})
    config.setdefault("lockfile", {})
    roots = source_roots(tree_paths(after))
    if roots:
        config["layering"] = {
            "acyclic": True, "layers": {"study-all": {"in": roots, "can_use": None}},
        }
    config_path.write_text(json.dumps(config, indent=2) + "\n", encoding="utf-8")
    return working, before, after, sorted(set(changed)), base, head


def fixture_excerpt(before: pathlib.Path, after: pathlib.Path, path: str, line: int | None) -> dict[str, object]:
    if not path:
        return {}
    start_line = line or 1
    def excerpt(root: pathlib.Path) -> str | None:
        target = root / path
        if not target.is_file():
            return None
        rows = target.read_text(encoding="utf-8", errors="replace").splitlines()
        return clean_evidence("\n".join(rows[max(0, start_line - 16):start_line + 15]))
    return {"path": path, "before": excerpt(before), "after": excerpt(after), "context": None}


def pilot_hard_negatives(cache: pathlib.Path) -> tuple[list[dict[str, object]], list[dict[str, object]]]:
    pilot = HERE.parent / "phenotype-pilot-2026-10-02"
    labels = {row["id"]: row for row in csv.DictReader((pilot / "labels.tsv").open(encoding="utf-8"), delimiter="\t")}
    worksheet = list(csv.DictReader((pilot / "worksheet.tsv").open(encoding="utf-8"), delimiter="\t"))
    selection = json.loads((pilot / "selection.json").read_text(encoding="utf-8"))
    changes = {
        f"{row['fullName'].replace('/', '__')}-{row['number']}": row
        for arm in ("agent", "human") for row in selection[arm]
    }
    family_map = {
        "klin:complexity": ("pilot-complexity", "shipped-complexity"),
        "klin:escapes": ("pilot-escapes", "shipped-escapes"),
        "klin:public-api": ("pilot-public-api", "shipped-public-api"),
    }
    placements = {
        row["id"]: row["candidate_placement"]
        for row in csv.DictReader((HERE / "phenotypes.tsv").open(encoding="utf-8"), delimiter="\t")
    }
    results, packets = [], []
    for row in worksheet:
        if labels.get(row["id"], {}).get("label") != "undesired" or row["family"] not in family_map:
            continue
        case_set_id, pid = family_map[row["family"]]
        change = changes[row["change"]]
        repo_name = str(change["fullName"])
        case_id = row["id"]
        path = "" if row["file"] == "-" else row["file"]
        line = int(row["line"] or 0) or None
        finding_id = row["id"]
        placement = placements[pid]
        result = "compulsory-repair" if placement == "Stop" else "non-compulsory-review"
        results.append({
            "study_version": 2, "case_set_id": case_set_id, "case_id": case_id,
            "phenotype_id": pid, "measurement_state": "complete",
            "finding_ids": [finding_id], "result": result,
            "reason": "frozen pilot finding labeled undesired; replayed as a classification control",
        })
        sample = {
            "language": str(change["language"]), "title": "", "body": "",
            "repository": repo_name, "author_login": "", "head_branch": "",
        }
        packets.append({
            "source_finding_id": f"hard-negative:{case_set_id}:{case_id}:{pid}:{finding_id}",
            "source_kind": "hard-negative", "case_set_id": case_set_id, "case_id": case_id,
            "finding_row_id": "", "change_id": "", "phenotype_id": pid,
            "finding_site": {
                "path": path, "start_line": line, "end_line": line,
                "start_byte": None, "end_byte": None, "owner": None,
            },
            "_change": sample,
            "_finding": {"finding_id": finding_id, "path": path, "line": line, "text": row["text"]},
            "_holes": ["the frozen pilot worksheet has no saved source excerpt or task context"],
            "_task_context": "Review the submitted change against the surrounding code.",
            "_diff_context": [],
        })
    return results, packets


def replay_fixture_hard_negatives(
    cache: pathlib.Path, klin: pathlib.Path, bins: dict[str, pathlib.Path],
    source: pathlib.Path, phenotypes: list[dict[str, str]],
) -> tuple[list[dict[str, object]], list[dict[str, object]]]:
    case_sets = {
        row["case_set_id"]: row for row in csv.DictReader(
            (HERE / "hard-negatives.tsv").open(encoding="utf-8"), delimiter="\t"
        )
    }
    registry = {row["id"]: row for row in phenotypes}
    results, packets = [], []
    gate_env = {key: value for key, value in os.environ.items() if not key.startswith("KLIN_")}
    gate_env.pop("GITHUB_EVENT_PATH", None)
    gate_env["GITHUB_BASE_REF"] = "main"
    for case_set_id, case_set in case_sets.items():
        if case_set_id.startswith("pilot-"):
            continue
        selected_phenotypes = case_set["applicable_phenotypes"].split("|")
        for case_id, base_source, route, language in fixture_routes(case_set_id):
            case_key = f"{case_set_id}:{case_id}"
            working, before, after, changed, base, head = fixture_tree(
                cache, case_set_id, case_id, base_source, route
            )
            proto_files = [
                path for path in changed if (after / path).is_file()
                and path.endswith((".rs", ".py", ".pyi", ".ts", ".tsx", ".mts", ".cts"))
            ]
            roots = source_roots(tree_paths(after))
            config_path = working / "klin.json"
            try:
                fixture_config = json.loads(config_path.read_text(encoding="utf-8"))
            except (OSError, json.JSONDecodeError):
                fixture_config = {}
            if not isinstance(fixture_config, dict):
                fixture_config = {}
            for section in ("inventory", "lockfile"):
                policy = fixture_config.get(section)
                if not isinstance(policy, dict) or not (policy.get("in") or policy.get("except")):
                    fixture_config[section] = {"except": ".git"}
            if roots and not isinstance(fixture_config.get("layering"), dict):
                fixture_config["layering"] = {
                    "acyclic": True,
                    "layers": {"study-all": {"in": roots, "can_use": None}},
                }
            config_path.write_text(json.dumps(fixture_config, indent=2) + "\n", encoding="utf-8")
            gate_result = command([str(klin), "gate", "--strict", "--json"], cwd=working, env=gate_env, timeout=900)
            try:
                gate_json = json.loads(gate_result.stdout)
            except json.JSONDecodeError:
                gate_json = None
            proto = {"asserts": [], "relations": [], "shapes": []}
            if proto_files:
                kind = "asserts" if language in ("Rust", "TypeScript") else "shapes"
                run = command([str(bins[kind]), "new", str(before), str(after), *proto_files], timeout=900)
                if run.returncode:
                    proto[kind + "_error"] = run.stderr.strip() or f"exit {run.returncode}"
                else:
                    proto[kind] = [line for line in run.stdout.splitlines() if line]
            if language in ("Rust", "TypeScript") and proto_files:
                run = command([str(bins["relations"]), "new", str(before), str(after)], timeout=900)
                if run.returncode:
                    proto["relations_error"] = run.stderr.strip() or f"exit {run.returncode}"
                else:
                    proto["relations"] = [line for line in run.stdout.splitlines() if line]
            dep = {"added": [], "lockshape": [], "added_error": None}
            if any(path.endswith(("pyproject.toml", "Cargo.toml", "package.json", "requirements.txt", "setup.cfg", "setup.py", "Pipfile")) for path in changed):
                dep_run = command([
                    sys.executable, str(source / "docs/dependency-evidence-2026-10-02/prototype/deps.py"),
                    "added", str(before), str(after),
                ], timeout=900)
                if dep_run.returncode:
                    dep["added_error"] = dep_run.stderr.strip() or f"exit {dep_run.returncode}"
                else:
                    dep["added"] = [line for line in dep_run.stdout.splitlines() if line]
            if any(path.endswith(("Cargo.lock", "package-lock.json")) for path in changed):
                dep_tool = source / "docs/dependency-evidence-2026-10-02/prototype/deps.py"
                old = command([sys.executable, str(dep_tool), "lockshape", str(before)], timeout=900)
                new = command([sys.executable, str(dep_tool), "lockshape", str(after)], timeout=900)
                if old.returncode or new.returncode:
                    dep["lockshape_error"] = old.stderr.strip() or new.stderr.strip() or "lockshape failed"
                else:
                    old_keys = {"\t".join(line.split("\t", 2)[:2]) for line in old.stdout.splitlines() if line}
                    dep["lockshape"] = [line for line in new.stdout.splitlines() if line and "\t" in line and "\t".join(line.split("\t", 2)[:2]) not in old_keys]

            for pid in selected_phenotypes:
                phenotype = registry[pid]
                if language not in phenotype["languages"].split("|"):
                    continue
                scoped = relevant(pid, language, changed)
                scoped = [path for path in scoped if (after / path).is_file()]
                state, holes, findings = "complete", [], []
                if not scoped:
                    state, holes = "unsupported", ["the selected route has no changed file in this phenotype's registered scope"]
                gate_name = GATE_BY_PHENOTYPE.get(pid)
                if state == "complete" and gate_name:
                    if gate_json is None:
                        state, holes = "tool-error", ["klin did not return a JSON report"]
                    else:
                        gate_row = gate_record(gate_json, gate_name)
                        if gate_row is None:
                            state, holes = "unsupported", [f"frozen klin did not run {gate_name}"]
                        else:
                            findings = [
                                {"finding_id": item.get("id", ""), "path": item.get("file"), "line": item.get("line"), "text": item.get("text", "")}
                                for item in parse_cli_findings(gate_json, gate_name)
                                if item.get("outcome") in {"new", "worsened"}
                            ]
                            unresolved = [
                                item for item in parse_cli_findings(gate_json, gate_name)
                                if item.get("outcome") in {"unparsed", "unresolved", "unknown"}
                            ]
                            coverage = gate_row.get("coverage") or {}
                            if unresolved or coverage.get("not_measured", 0) or coverage.get("unreadable", 0):
                                state = "partial"
                                holes.extend(
                                    f"{item.get('file', '')}:{item.get('line', '')}: {item.get('outcome')}"
                                    for item in unresolved
                                )
                                if coverage.get("not_measured", 0) or coverage.get("unreadable", 0):
                                    holes.append(f"{gate_name} has unmeasured or unreadable coverage")
                elif state == "complete" and pid in ASSERT_CANDIDATES:
                    findings = proto_finding(pid, proto["asserts"])
                    if "asserts_error" in proto:
                        state, holes = "tool-error", [str(proto["asserts_error"])]
                elif state == "complete" and pid in SHAPE_CANDIDATES:
                    findings = proto_finding(pid, proto["shapes"])
                    if "shapes_error" in proto:
                        state, holes = "tool-error", [str(proto["shapes_error"])]
                elif state == "complete" and pid in RELATION_CANDIDATES:
                    findings = proto_finding(pid, proto["relations"])
                    if "relations_error" in proto:
                        state, holes = "tool-error", [str(proto["relations_error"])]
                    elif pid == "design-component-cycle" and language == "TypeScript" and any("unknown" in line for line in proto["relations"]):
                        state = "local-resolution-incomplete"
                        holes.append("the frozen ModuleGraph prototype reports unresolved local dependency evidence")
                elif state == "complete" and pid == "new-direct-dependency":
                    for line in dep["added"]:
                        fields = line.split("\t")
                        if len(fields) >= 3:
                            findings.append({"finding_id": f"{fields[0]}:{fields[1]}", "path": fields[2], "line": None, "text": f"new direct dependency {fields[0]}:{fields[1]}"})
                    if dep["added_error"]:
                        state, holes = "tool-error", [str(dep["added_error"])]
                elif state == "complete" and pid == "cargo-lock-entry-shape":
                    findings = [
                        {"finding_id": ":".join(line.split("\t", 2)), "path": line.split("\t", 2)[0], "line": None, "text": line.split("\t", 2)[-1]}
                        for line in dep["lockshape"] if len(line.split("\t")) >= 3
                    ]
                    if dep.get("lockshape_error"):
                        state, holes = "tool-error", [str(dep["lockshape_error"])]
                elif state == "complete" and pid == "python-uv-lock":
                    findings, state, holes, _ = uv_lock_findings(before, after, scoped)
                elif state == "complete" and pid in {"ruff-python-injection", "ruff-python-swallowed"}:
                    findings, state, holes, _ = ruff_findings(working, scoped, pid, cache)
                if pid == "shipped-module-cycle" and not roots and state == "complete":
                    state, holes = "unsupported", ["the frozen survey found no source root for layering"]
                ids = [
                    f"{item.get('finding_id', 'finding')}:{ordinal:05d}"
                    for ordinal, item in enumerate(findings, 1)
                ]
                if state != "complete":
                    result = "measurement-incomplete"
                elif not findings:
                    result = "no-finding"
                elif phenotype["candidate_placement"] == "Stop":
                    result = "compulsory-repair"
                else:
                    result = "non-compulsory-review"
                results.append({
                    "study_version": 2, "case_set_id": case_set_id, "case_id": case_id,
                    "phenotype_id": pid, "measurement_state": state,
                    "finding_ids": ids, "result": result,
                    "reason": "; ".join(holes) if holes else (
                        "registered Stop placement would compel repair on this frozen hard-negative route"
                        if result == "compulsory-repair" else "frozen hard-negative route replay"
                    ),
                })
                for ordinal, finding in enumerate(findings, 1):
                    path = str(finding.get("path") or "")
                    line = int(finding["line"]) if str(finding.get("line", "")).isdigit() else None
                    finding_id = f"{finding.get('finding_id', 'finding')}:{ordinal:05d}"
                    packets.append({
                        "source_finding_id": f"hard-negative:{case_set_id}:{case_id}:{pid}:{finding_id}",
                        "source_kind": "hard-negative", "case_set_id": case_set_id, "case_id": case_id,
                        "finding_row_id": "", "change_id": "", "phenotype_id": pid,
                        "finding_site": {
                            "path": path, "start_line": line, "end_line": line,
                            "start_byte": None, "end_byte": None, "owner": None,
                        },
                        "_change": {"language": language, "repository": "", "title": "", "body": "", "author_login": "", "head_branch": ""},
                        "_finding": finding,
                        "_holes": holes + [
                            "the frozen case source has no route-level task description"
                        ],
                        "_task_context": "Review the submitted change against its base tree and visible test intent.",
                        "_diff_context": [fixture_excerpt(before, after, path, line)] if path else [],
                    })
            print(f"hard-negative {case_set_id} {case_id}", flush=True)
    pilot_results, pilot_packets = pilot_hard_negatives(cache)
    results.extend(pilot_results)
    packets.extend(pilot_packets)
    return results, packets


def decoded_json(value: object, fallback: object) -> object:
    if isinstance(value, str):
        try:
            return json.loads(value)
        except json.JSONDecodeError:
            return fallback
    return value


def report_record(path: pathlib.Path) -> dict[str, object] | None:
    if not path.is_file():
        return None
    try:
        value = json.loads(path.read_text(encoding="utf-8"))
    except json.JSONDecodeError:
        return None
    report_value = value.get("report") if isinstance(value, dict) else None
    return report_value if isinstance(report_value, dict) else None


def measurement_gate_finding_count(report_value: dict[str, object] | None, gate_name: str) -> int:
    if report_value is None:
        return 0
    findings = report_value.get("findings", [])
    return sum(
        isinstance(item, dict)
        and item.get("gate") == gate_name
        and item.get("outcome") in {"new", "worsened"}
        for item in findings if isinstance(findings, list)
    )


def detector_outcome_known(state: str, finding_count: int) -> bool:
    # A positive is known even when other sites remain unresolved. A negative is known
    # only when the final measurement row is complete; partial and unsupported rows
    # cannot contribute a clean result to the change-level rate.
    return finding_count > 0 or state == "complete"


def reconcile_measurements(rows: list[dict[str, object]], cache: pathlib.Path) -> None:
    """Replace whole-tree/file coverage metrics with registered-scope censuses.

    Some frozen detectors do not expose their registered eligible-unit census. Those rows keep
    their observed findings, but carry a partial state and cannot produce a site prevalence rate.
    """
    samples_path = HERE / "natural-sample.tsv"
    samples = list(csv.DictReader(samples_path.open(encoding="utf-8"), delimiter="\t"))
    by_change = {str(sample["change_id"]): sample for sample in samples}
    phenotype_languages = {
        row["id"]: set(row["languages"].split("|"))
        for row in csv.DictReader((HERE / "phenotypes.tsv").open(encoding="utf-8"), delimiter="\t")
    }
    replay = cache / "study-v2" / "replay"
    scopes: dict[str, dict[str, object] | None] = {}
    full_reports: dict[str, dict[str, object] | None] = {}
    paths_by_change: dict[str, tuple[pathlib.Path, list[str]]] = {}
    head_paths_by_change: dict[str, set[str]] = {}

    for change_id, sample in by_change.items():
        bare = bare_repository(cache, str(sample["repository"]))
        if not bare.is_dir():
            paths_by_change[change_id] = (bare, [])
            scopes[change_id] = None
            full_reports[change_id] = None
            continue
        paths = changed_paths(bare, str(sample["base"]), str(sample["head"]))
        head_paths = all_paths(bare, str(sample["head"]))
        paths_by_change[change_id] = (bare, paths)
        head_paths_by_change[change_id] = set(head_paths)
        scopes[change_id] = report_record(replay / "runs" / f"{change_id}-scope.json")
        full_reports[change_id] = report_record(replay / "runs" / f"{change_id}.json")

    for row in rows:
        change_id = str(row["change_id"])
        sample = by_change.get(change_id)
        values = decoded_json(row.get("values", {}), {})
        values = values if isinstance(values, dict) else {}
        holes = decoded_json(row.get("holes", []), [])
        holes = list(holes) if isinstance(holes, list) else []
        phenotype = str(row["phenotype_id"])
        semantic = row.get("semantic_eligible") is True or str(row.get("semantic_eligible")) == "True"
        finding_count = int(values.get("finding_count", 0) or 0)
        lockfile_census_path = replay / "runs" / f"{change_id}-lockfile-census.json"
        try:
            lockfile_census = json.loads(lockfile_census_path.read_text(encoding="utf-8"))
        except (OSError, json.JSONDecodeError):
            lockfile_census = {}
        units: int | None = None
        unit_census_known: bool | None = None
        method = ""

        scope_files: list[str] = []
        if sample is not None:
            bare, changed = paths_by_change[change_id]
            recorded_scope_files = values.get("measurement_scope_files", [])
            scope_files = (
                relevant(phenotype, str(sample["language"]), changed)
                if bare.is_dir()
                else recorded_scope_files if isinstance(recorded_scope_files, list) else []
            )
            if phenotype in {"ruff-python-injection", "ruff-python-swallowed"} and bare.is_dir():
                scope_files = [path for path in scope_files if path in head_paths_by_change[change_id]]
            values["measurement_scope_files"] = scope_files

        if sample is not None and phenotype == "shipped-complexity":
            existing = int(row.get("eligibility_count", 0) or 0)
            has_finding = measurement_gate_finding_count(full_reports.get(change_id), "complexity") > 0
            gate = gate_record(scopes.get(change_id), "complexity")
            coverage = (gate or {}).get("coverage") or {}
            incomplete = bool(int(coverage.get("not_measured", 0) or 0) or int(coverage.get("unreadable", 0) or 0))
            if existing > 0:
                units, method = existing, "changed functions parsed and judged by frozen klin complexity"
                unit_census_known = not incomplete
            elif semantic or has_finding:
                semantic = True
                row["semantic_eligible"] = True
                if has_finding or int(coverage.get("found", 0)) > 0:
                    method = "changed functions could not be enumerated by the failing/empty CLI summary"
            elif not incomplete:
                units, unit_census_known = 0, True
                method = "no changed complexity function in the frozen scope"

        elif sample is not None and phenotype == "shipped-escapes":
            bare, _paths = paths_by_change[change_id]
            scope_files = values.get("measurement_scope_files", [])
            if bare.is_dir() and isinstance(scope_files, list):
                units = diff_line_count(bare, str(sample["base"]), str(sample["head"]), [str(path) for path in scope_files])
                method = "added and deleted source-line events in supported changed files"
                unit_census_known = True

        elif sample is not None and phenotype == "shipped-lockfile":
            manifests = lockfile_census.get("eligible_manifests", [])
            values["eligible_manifest_paths"] = manifests if isinstance(manifests, list) else []
            if lockfile_census.get("eligible_pair") is not True:
                units, unit_census_known = 0, True
                method = "no changed manifest-lock pair in the frozen Rust/npm scope"
            elif isinstance(lockfile_census.get("eligible_units"), int):
                units = int(lockfile_census["eligible_units"])
                unit_census_known = lockfile_census.get("unit_census_complete") is True
                method = "direct dependency entries in changed supported manifest-lock pairs counted by the frozen CLI"
            else:
                method = "frozen CLI did not expose a direct dependency count for every affected manifest"
                unit_census_known = False
            if lockfile_census.get("eligible_pair") is True and not unit_census_known:
                if str(row.get("measurement_state")) == "complete":
                    row["measurement_state"] = "partial"
                if lockfile_census.get("deleted_manifest_pair") is True:
                    holes.append("a deleted manifest's before-tree dependency sites are not exposed by the frozen lockfile CLI")
                else:
                    holes.append("the frozen lockfile CLI did not expose a complete eligible-unit census for dependency sites")

        elif sample is not None and phenotype == "shipped-test-deletion":
            gate = gate_record(scopes.get(change_id), "inventory")
            if gate is not None:
                coverage = gate.get("coverage") or {}
                finding_count = measurement_gate_finding_count(scopes.get(change_id), "inventory")
                census_hole = int(coverage.get("not_measured", 0)) or int(coverage.get("unreadable", 0))
                unresolved_identity = any(
                    any(word in str(hole).lower() for word in ("unresolved", "unparsed", "unknown"))
                    for hole in holes
                )
                if not census_hole:
                    units = int(gate.get("held") or 0) + finding_count
                    method = "before-tree test identities held or removed in changed scope"
                    unit_census_known = not unresolved_identity
                else:
                    method = "inventory could not census every before-tree test identity in changed scope"
                    units = int(gate.get("held") or 0) + finding_count
                    unit_census_known = False

        elif sample is not None and phenotype == "shipped-reachability":
            method = "frozen project module-discovery membership is not exposed in the replay record"

        elif sample is not None and phenotype in {"ruff-python-injection", "ruff-python-swallowed"}:
            if isinstance(scope_files, list):
                units = len(scope_files)
                method = "changed Python files passed to the registered Ruff family"
                unit_census_known = True

        elif sample is not None and phenotype == "new-direct-dependency":
            dependency_path = replay / "runs" / f"{change_id}-dependencies.json"
            try:
                dependency = json.loads(dependency_path.read_text(encoding="utf-8"))
            except (OSError, json.JSONDecodeError):
                dependency = {}
            if not dependency.get("added_error") and isinstance(dependency.get("added"), list):
                manifest_paths = dependency.get("manifest_paths", [])
                values["eligible_manifest_paths"] = manifest_paths if isinstance(manifest_paths, list) else []
                method = "full changed-manifest dependency-name census is unavailable"
                # The frozen prototype emits additions but does not report parse coverage for
                # every supported manifest. A finding proves a positive change outcome; an
                # empty result cannot prove that no dependency was added.
                if values["eligible_manifest_paths"] and finding_count == 0:
                    hole = "the frozen dependency prototype does not expose changed-manifest parse coverage; a no-finding outcome is unresolved"
                    if hole not in holes:
                        holes.append(hole)
                if values["eligible_manifest_paths"] and row.get("measurement_state") == "unsupported":
                    row["measurement_state"] = "partial"
            elif scope_files:
                row["measurement_state"] = "tool-error"
                row["measured_count"] = 0
                error = dependency.get("added_error") or "cached dependency parser result is missing"
                holes.append(str(error))
                values["tool_failure"] = True
                values["change_outcome_known"] = False

        supported_language = sample is not None and str(sample["language"]) in phenotype_languages.get(phenotype, set())
        change_eligibility: bool | None = None
        change_method = "registered change-level eligibility is not exposed by the frozen detector"

        if sample is not None and not supported_language:
            change_eligibility, change_method = False, "language is outside the frozen phenotype scope"
        elif sample is not None and not paths_by_change[change_id][0].is_dir():
            change_method = "repository scope cache is unavailable"
        elif sample is not None and phenotype in {"shipped-escapes", "shipped-stubs"}:
            change_eligibility = bool(scope_files)
            change_method = "changed supported source-file scope"
        elif sample is not None and phenotype in {"ruff-python-injection", "ruff-python-swallowed"}:
            change_eligibility = bool(scope_files)
            change_method = "changed Python files passed to the exact Ruff recipe"
        elif sample is not None and phenotype == "shipped-lockfile":
            change_eligibility = lockfile_census.get("eligible_pair") is True
            change_method = "the change touches a discovered manifest and its supported Rust/npm lock pair"
        elif sample is not None and phenotype == "new-direct-dependency":
            change_eligibility = bool(values.get("eligible_manifest_paths", []))
            change_method = "changed supported direct-dependency manifest path"
        elif sample is not None and phenotype == "shipped-reachability":
            if int(values.get("finding_count", 0) or 0) > 0:
                change_eligibility = True
                change_method = "a frozen reachability finding proves an eligible candidate"
            elif not scope_files:
                change_eligibility = False
                change_method = "no changed supported source file in the registered candidate scope"
            else:
                change_method = "frozen project module-discovery membership is not exposed in the replay record"
        elif sample is not None and phenotype == "shipped-complexity":
            gate = gate_record(scopes.get(change_id), "complexity")
            coverage = (gate or {}).get("coverage") or {}
            census_hole = bool(int(coverage.get("not_measured", 0) or 0) or int(coverage.get("unreadable", 0) or 0))
            if units is not None and units > 0:
                change_eligibility = True
                change_method = "at least one changed complexity function enumerated by the frozen CLI"
            elif unit_census_known is True and not census_hole:
                change_eligibility = False
                change_method = "no changed complexity function in the frozen scope"
        elif sample is not None and phenotype == "shipped-test-deletion":
            gate = gate_record(scopes.get(change_id), "inventory")
            coverage = (gate or {}).get("coverage") or {}
            census_hole = bool(int(coverage.get("not_measured", 0) or 0) or int(coverage.get("unreadable", 0) or 0))
            if units is not None and units > 0:
                change_eligibility = True
                change_method = "at least one before-tree test identity is present in changed scope"
            elif unit_census_known is True and not census_hole:
                change_eligibility = False
                change_method = "no before-tree test identity is present in changed scope"
        elif sample is not None and int(values.get("finding_count", 0) or 0) > 0:
            change_eligibility, change_method = True, "a frozen detector finding proves an eligible candidate"
        elif sample is not None and not scope_files:
            change_eligibility, change_method = False, "no changed file in the registered candidate scope"

        if change_eligibility is not None:
            semantic = change_eligibility
        elif sample is not None and scope_files:
            semantic = True
        row["semantic_eligible"] = semantic
        values["eligible_changes_known"] = change_eligibility is not None
        values["eligible_change_method"] = change_method
        if units is not None:
            row["semantic_eligible"] = semantic
            row["eligibility_count"] = units
            values["eligible_units"] = units
            values["eligible_units_known"] = unit_census_known is True
            values["eligibility_unit_method"] = method
            values["eligibility_change_method"] = change_method
            if str(row.get("measurement_state")) == "complete" and unit_census_known is not False:
                row["measured_count"] = units
            elif semantic and unit_census_known is False:
                if str(row.get("measurement_state")) == "complete":
                    row["measurement_state"] = "partial"
                hole = "registered eligible-unit census is incomplete; site prevalence is not estimated"
                if hole not in holes:
                    holes.append(hole)
        elif semantic:
            row["eligibility_count"] = 0
            row["measured_count"] = 0
            values["eligible_units"] = 0
            values["eligible_units_known"] = False
            values["eligibility_unit_method"] = method or "registered eligible-unit census unavailable"
            values["eligibility_change_method"] = change_method
            hole = "registered eligible-unit census is unavailable; site prevalence is not estimated"
            if hole not in holes:
                holes.append(hole)
            if str(row.get("measurement_state")) == "complete":
                row["measurement_state"] = "partial"
        else:
            row["eligibility_count"] = 0
            row["measured_count"] = 0
            values["eligible_units"] = 0
            values["eligible_units_known"] = True
            values["eligibility_unit_method"] = "no registered unit in the changed scope"
            values["eligibility_change_method"] = change_method

        values["change_outcome_known"] = detector_outcome_known(
            str(row.get("measurement_state", "")), finding_count
        )
        row["holes"] = holes
        row["values"] = values


def reconcile_saved_measurements(cache: pathlib.Path) -> None:
    csv.field_size_limit(10_000_000)
    path = HERE / "measurements.tsv"
    if not path.is_file():
        raise StudyError("measurements.tsv is missing; run replay first")
    rows = list(csv.DictReader(path.open(encoding="utf-8"), delimiter="\t"))
    for row in rows:
        row["eligibility_count"] = int(row["eligibility_count"] or 0)
        row["measured_count"] = int(row["measured_count"] or 0)
        row["holes"] = decoded_json(row.get("holes", "[]"), [])
        row["values"] = decoded_json(row.get("values", "{}"), {})
        row["semantic_eligible"] = str(row["semantic_eligible"]).lower() == "true"
    reconcile_measurements(rows, cache)
    write_json_tsv(path, MEASUREMENT_FIELDS, rows)
    report(cache)


def report(cache: pathlib.Path | None = None) -> None:
    csv.field_size_limit(10_000_000)
    measurements_path, findings_path = HERE / "measurements.tsv", HERE / "findings.tsv"
    if not measurements_path.is_file() or not findings_path.is_file():
        raise StudyError("measurements.tsv or findings.tsv is missing; run replay first")
    measurements = list(csv.DictReader(measurements_path.open(encoding="utf-8"), delimiter="\t"))
    findings = list(csv.DictReader(findings_path.open(encoding="utf-8"), delimiter="\t"))
    counts: dict[tuple[str, str], list[dict[str, str]]] = {}
    for row in measurements:
        counts.setdefault((row["phenotype_id"], row["population_id"]), []).append(row)
    finding_counts: dict[str, int] = {}
    for row in findings:
        finding_counts[row["measurement_row_id"]] = finding_counts.get(row["measurement_row_id"], 0) + 1
    lines = [
        "# Descriptive natural prevalence",
        "",
        "The natural-agent and matched-human arms are shown separately. Human controls are changes not attributed to an agent by the preregistered metadata exclusions; this is not proof of human-only authorship.",
        "Rate differences are descriptive and do not establish an AI-specific effect.",
        "",
        "## Materialized sample",
        "",
        "| Language | Agent changes | Matched human changes | Unmatched agent changes |",
        "| --- | ---: | ---: | ---: |",
    ]
    sample_path = HERE / "natural-sample.tsv"
    if sample_path.is_file():
        samples = list(csv.DictReader(sample_path.open(encoding="utf-8"), delimiter="\t"))
        for language in LANGUAGES:
            agents = [row for row in samples if row["language"] == language and row["population"] == "natural-agent"]
            humans = [row for row in samples if row["language"] == language and row["population"] == "matched-human"]
            lines.append(
                f"| {language} | {len(agents)} | {len(humans)} | "
                f"{sum(row['match_status'] != 'matched' for row in agents)} |"
            )
    lines += [
        "",
        "Frozen detector replay only. These are descriptive measurements, not valid-regression labels or product dispositions.",
        "Incomplete rows remain visible and are not counted as clean. When the change census is complete, an affected-change interval includes observed positives as its lower bound and unresolved eligible changes as its upper bound. Change rates are withheld when change-level eligibility is unresolved. Site rates are withheld when change eligibility, any eligible-unit census, or any eligible measurement is incomplete.",
        "",
        "| Phenotype | Population | Affected / eligible changes | Findings / eligible units | Change eligibility known / sampled changes | Unit census known / possible eligible changes | Fully measured / eligible changes | Measured / eligible units | State counts |",
        "| --- | --- | ---: | ---: | ---: | ---: | ---: | ---: | --- |",
    ]
    phenotypes = [line.split("\t", 1)[0] for line in (HERE / "phenotypes.tsv").read_text().splitlines()[1:]]
    populations = ["natural-agent", "matched-human"]
    for pid in phenotypes:
        for population in populations:
            rows = counts.get((pid, population), [])
            eligible = [row for row in rows if row["semantic_eligible"] == "True"]
            eligible_changes = len(eligible)
            values_by_row = {
                row["row_id"]: decoded_json(row.get("values", "{}"), {})
                for row in rows
            }
            known_change_count = sum(
                isinstance(values_by_row[row["row_id"]], dict)
                and values_by_row[row["row_id"]].get("eligible_changes_known") is True
                for row in rows
            )
            change_census_known = known_change_count == len(rows)
            units_known = [
                row for row in eligible
                if isinstance(values_by_row[row["row_id"]], dict)
                and values_by_row[row["row_id"]].get("eligible_units_known") is True
            ]
            eligible_units = sum(int(row["eligibility_count"]) for row in units_known)
            measured_changes = sum(row["measurement_state"] == "complete" for row in eligible)
            affected_changes = sum(finding_counts.get(row["row_id"], 0) > 0 for row in eligible)
            unresolved_changes = sum(
                values_by_row[row["row_id"]].get("change_outcome_known") is not True
                and finding_counts.get(row["row_id"], 0) == 0
                for row in eligible
            )
            if not change_census_known:
                change_rate = (
                    f"unavailable ({affected_changes} observed; eligibility unresolved for "
                    f"{len(rows) - known_change_count})"
                )
            elif not eligible_changes:
                change_rate = "no eligible changes"
            elif unresolved_changes:
                change_rate = f"{affected_changes}–{affected_changes + unresolved_changes} / {eligible_changes}"
            else:
                change_rate = f"{affected_changes} / {eligible_changes}"
            affected_sites = sum(finding_counts.get(row["row_id"], 0) for row in eligible)
            exact_sites = (
                change_census_known
                and len(units_known) == eligible_changes
                and measured_changes == eligible_changes
            )
            measured_units = sum(int(row["measured_count"]) for row in units_known)
            site_rate = (
                f"{affected_sites} / {eligible_units}"
                if exact_sites and eligible_changes else "unavailable"
            )
            unit_rate = (
                f"{measured_units} / {eligible_units}"
                if exact_sites and eligible_changes else "unavailable"
            )
            fully_measured = (
                f"{measured_changes} / {eligible_changes}"
                if change_census_known else "unavailable"
            )
            states: dict[str, int] = {}
            for row in eligible:
                states[row["measurement_state"]] = states.get(row["measurement_state"], 0) + 1
            lines.append(
                f"| {pid} | {population} | {change_rate} | "
                f"{site_rate} | {known_change_count} / {len(rows)} | {len(units_known)} / {eligible_changes} | {fully_measured} | "
                f"{unit_rate} | {', '.join(f'{key}={value}' for key, value in sorted(states.items())) or '—'} |"
            )
    lines += ["", "All counts can be recalculated from `measurements.tsv` and `findings.tsv`.", ""]
    (HERE / "prevalence-descriptive.md").write_text("\n".join(lines))


def main(argv: list[str]) -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    sub = parser.add_subparsers(dest="command", required=True)
    materialize_parser = sub.add_parser("materialize", help="select agent and matched-human changes")
    materialize_parser.add_argument("--cache", type=pathlib.Path, required=True)
    rematch_parser = sub.add_parser("rematch-humans", help="rebuild matched controls from a saved agent sample")
    rematch_parser.add_argument("--cache", type=pathlib.Path, required=True)
    build_parser = sub.add_parser("build-tools", help="build the frozen study binary and prototypes")
    build_parser.add_argument("--cache", type=pathlib.Path, required=True)
    replay_parser = sub.add_parser("replay", help="measure every selected natural change")
    replay_parser.add_argument("--cache", type=pathlib.Path, required=True)
    reconcile_parser = sub.add_parser("reconcile", help="refresh registered-unit census and descriptive rates")
    reconcile_parser.add_argument("--cache", type=pathlib.Path, required=True)
    report_parser = sub.add_parser("report", help="recalculate descriptive prevalence")
    report_parser.add_argument("--cache", type=pathlib.Path)
    args = parser.parse_args(argv)
    if args.command == "materialize":
        materialize(args.cache)
        return 0
    if args.command == "rematch-humans":
        rematch_humans(args.cache)
        return 0
    if args.command == "build-tools":
        build_tools(args.cache)
        return 0
    if args.command == "replay":
        replay(args.cache)
        return 0
    if args.command == "reconcile":
        reconcile_saved_measurements(args.cache)
        return 0
    if args.command == "report":
        report(args.cache)
        return 0
    return 2


if __name__ == "__main__":
    try:
        raise SystemExit(main(sys.argv[1:]))
    except StudyError as error:
        print(f"study: {error}", file=sys.stderr)
        raise SystemExit(2)
