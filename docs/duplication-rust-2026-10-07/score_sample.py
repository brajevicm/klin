"""Score independent labels for the frozen #494 Rust sample with stratum weights."""
import json
import sys
from pathlib import Path

HERE = Path(__file__).parent
LABELS = {"copy", "boilerplate", "required-shape", "generated", "distinct", "mixed"}


def estimate(labels, strata, language):
    copies = {True: 0.0, False: 0.0}
    population = {True: 0, False: 0}
    for stratum in strata:
        if stratum["language"] != language:
            continue
        kept = stratum["candidate_kept"]
        ids = stratum["sample_ids"]
        n = len(ids)
        assert n == stratum["sample_n"] > 0
        population[kept] += stratum["population_n"]
        copies[kept] += sum(labels[pair_id]["label"] == "copy" for pair_id in ids) * stratum["population_n"] / n
    precision = copies[True] / population[True] if population[True] else None
    total_copies = copies[True] + copies[False]
    recall = copies[True] / total_copies if total_copies else None
    return {
        "kept_population": population[True],
        "estimated_kept_copies": copies[True],
        "estimated_all_copies": total_copies,
        "precision": precision,
        "recall": recall,
    }


def read_labels(path, expected_ids):
    labels = json.loads(Path(path).read_text())
    assert set(labels) == expected_ids, f"{path}: label ids differ from the frozen packet"
    assert all(value.get("label") in LABELS for value in labels.values()), f"{path}: invalid label"
    return labels


def self_test():
    strata = [
        {"language": "Rust", "candidate_kept": True, "population_n": 100, "sample_n": 2, "sample_ids": ["a", "b"]},
        {"language": "Rust", "candidate_kept": False, "population_n": 300, "sample_n": 3, "sample_ids": ["c", "d", "e"]},
    ]
    labels = {key: {"label": label} for key, label in {
        "a": "copy", "b": "boilerplate", "c": "copy", "d": "distinct", "e": "boilerplate",
    }.items()}
    result = estimate(labels, strata, "Rust")
    assert result["estimated_kept_copies"] == 50
    assert result["estimated_all_copies"] == 150
    assert result["precision"] == 0.5 and abs(result["recall"] - 1 / 3) < 1e-12


def main(paths):
    packet = json.loads((HERE / "review/review-packet.json").read_text())
    expected_ids = {row["id"] for row in packet}
    strata = json.loads((HERE / "sampling.json").read_text())["strata"]
    reviewers = {Path(path).stem: read_labels(path, expected_ids) for path in paths}
    result = {
        name: {"Rust": estimate(labels, strata, "Rust")}
        for name, labels in reviewers.items()
    }
    for metrics in result.values():
        rust = metrics["Rust"]
        rust["passes_494"] = rust["precision"] is not None and rust["recall"] is not None and rust["precision"] >= 0.8 and rust["recall"] >= 0.4
    if len(reviewers) == 2:
        first, second = reviewers.values()
        exact = sum(first[pair_id]["label"] == second[pair_id]["label"] for pair_id in expected_ids)
        same_copy = sum((first[pair_id]["label"] == "copy") == (second[pair_id]["label"] == "copy") for pair_id in expected_ids)
        p1 = sum(value["label"] == "copy" for value in first.values()) / len(expected_ids)
        p2 = sum(value["label"] == "copy" for value in second.values()) / len(expected_ids)
        observed, expected = same_copy / len(expected_ids), p1 * p2 + (1 - p1) * (1 - p2)
        result["agreement"] = {
            "pairs": len(expected_ids),
            "same_label": exact / len(expected_ids),
            "same_copy_decision": observed,
            "copy_kappa": (observed - expected) / (1 - expected) if expected < 1 else None,
        }
    print(json.dumps(result, indent=2))


if __name__ == "__main__":
    if sys.argv[1:] == ["--self-test"]:
        self_test()
        print("weighted estimate self-test passed")
    else:
        assert len(sys.argv) in (2, 3), "usage: score_sample.py REVIEWER_A.json [REVIEWER_B.json]"
        main(sys.argv[1:])
