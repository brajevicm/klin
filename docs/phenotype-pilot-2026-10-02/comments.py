"""Read the entered review comments of every selected pull request.

usage: python3 comments.py
Writes comments.json: one entry per entered comment, keyed by a stable id.
"""

import json
import pathlib
import subprocess

HERE = pathlib.Path(__file__).resolve().parent
LIMIT = 2000


def pages(path):
    done = subprocess.run(["gh", "api", "--paginate", "--slurp", path], capture_output=True, text=True, check=True)
    return [item for page in json.loads(done.stdout) for item in page]


def entered(user, author, body):
    return (
        user is not None
        and user["login"] != author
        and user["type"] != "Bot"
        and not user["login"].endswith("[bot]")
        and user["login"] != "Copilot"
        and bool((body or "").strip())
    )


def main():
    selection = json.loads((HERE / "selection.json").read_text())
    out = []
    for change in selection["agent"] + selection["human"]:
        name, number, author = change["fullName"], change["number"], change["author"]
        sources = [
            ("issue", pages(f"repos/{name}/issues/{number}/comments?per_page=100")),
            ("review", pages(f"repos/{name}/pulls/{number}/reviews?per_page=100")),
            ("line", pages(f"repos/{name}/pulls/{number}/comments?per_page=100")),
        ]
        for kind, items in sources:
            for item in items:
                if not entered(item.get("user"), author, item.get("body")):
                    continue
                out.append(
                    {
                        "id": f"{name}#{number}/{kind}/{item['id']}",
                        "repository": name,
                        "number": number,
                        "kind": kind,
                        "author": item["user"]["login"],
                        "path": item.get("path"),
                        "body": item["body"][:LIMIT],
                    }
                )
    (HERE / "comments.json").write_text(json.dumps(out, indent=2, ensure_ascii=False) + "\n")
    print(f"{len(out)} comments")


if __name__ == "__main__":
    main()
