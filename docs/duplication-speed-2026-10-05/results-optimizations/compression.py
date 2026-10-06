"""Lossless equality-artifact size experiment. Stdlib, frozen normalizer CLI."""
import argparse
import json
import pathlib
import subprocess


def varint(n):
    out = bytearray()
    while n > 127:
        out.append((n & 127) | 128)
        n >>= 7
    out.append(n)
    return bytes(out)


def numbers(data):
    n = shift = 0
    for byte in data:
        n |= (byte & 127) << shift
        if byte < 128:
            yield n
            n = shift = 0
        else:
            shift += 7
    assert shift == 0, "truncated varint"


def encode(tokens, safe, dictionary, inverse=None):
    # ID zero is a hard unsafe boundary, never an equality witness.
    out = bytearray()
    for token, ok in zip(tokens, safe):
        if ok and token not in dictionary:
            dictionary[token] = len(dictionary) + 1
            if inverse is not None:
                inverse.append(token)
        out.extend(varint(dictionary[token] if ok else 0))
    return bytes(out)


def decode(chain, dictionary, inverse=None):
    if inverse is None:
        inverse = {v: k for k, v in dictionary.items()}
    return [inverse[n] if n else None for n in numbers(chain)]


def check():
    for n in (0, 1, 127, 128, 16383, 16384, 2**32):
        assert list(numbers(varint(n))) == [n]
    left, right = {}, {}
    a = encode([b"rust:x", b"ts:x@pkg", b"bad"], [True, True, False], left)
    b = encode([b"ts:x@pkg", b"rust:x", b"bad"], [True, True, False], right)
    assert a == b  # Local IDs are NOT comparable across trees.
    assert decode(a, left) != decode(b, right)
    shared = {}
    a = encode([b"rust:x", b"ts:x@pkg", b"bad"], [True, True, False], shared)
    b = encode([b"ts:x@pkg", b"rust:x", b"bad"], [True, True, False], shared)
    assert a != b
    assert decode(a, shared) == [b"rust:x", b"ts:x@pkg", None]
    assert encode([], [], shared) == b""  # Excluded test units add nothing.


def eligible(path):
    return (path.suffix in (".rs", ".ts", ".tsx") and not path.name.endswith(".d.ts")
            and not any(p in ("tests", "test", "__tests__", "benches", "fixtures") for p in path.parts)
            and not path.name.endswith("_test.rs") and path.name != "tests.rs"
            and not any(s in str(path) for s in (".test.", ".spec."))
            and not any(p.startswith(".") or p in ("node_modules", "target", "dist", "build") for p in path.parts))


def measure(root, binary, indexes):
    dictionary = {}
    inverse = [None]
    chain_bytes = row_bytes = paths_bytes = tokens = unsafe = files = 0
    for path in sorted(root.rglob("*")):
        if not path.is_file() or not eligible(path.relative_to(root)):
            continue
        data = json.loads(subprocess.check_output([str(binary), "normalize", str(path.relative_to(root))], cwd=root))
        raw = [bytes.fromhex(t) for t in data["tokens"]]
        encoded = encode(raw, data["safe"], dictionary, inverse)
        assert decode(encoded, dictionary, inverse) == [t if ok else None for t, ok in zip(raw, data["safe"])]
        rows = data["rows"]
        row_bytes += sum(len(varint(row - (rows[i - 1] if i else 0))) for i, row in enumerate(rows))
        chain_bytes += len(encoded)
        relative = str(path.relative_to(root)).encode()
        paths_bytes += len(varint(len(relative))) + len(relative) + len(varint(len(raw)))
        tokens += len(raw)
        unsafe += sum(not ok for ok in data["safe"])
        files += 1
    dictionary_data = varint(len(dictionary)) + b"".join(varint(len(t)) + t for t in dictionary)
    dictionary_bytes = len(dictionary_data)
    result = dict(root=str(root), files=files, tokens=tokens, unsafe_tokens=unsafe,
                  unique_safe_tokens=len(dictionary), dictionary_bytes=dictionary_bytes,
                  token_chain_bytes=chain_bytes, row_delta_bytes=row_bytes, paths_bytes=paths_bytes,
                  equality_bytes=dictionary_bytes + chain_bytes,
                  reporting_bytes=dictionary_bytes + chain_bytes + row_bytes + paths_bytes,
                  lineage_bytes=None, timing_claim=False)
    result["candidate_totals"] = {}
    for index in indexes:
        raw = (index / "index.bin").stat().st_size
        result["candidate_totals"][index.name] = dict(candidate_with_paths_bytes=raw,
            candidate_plus_equality_bytes=raw + dictionary_bytes + chain_bytes,
            candidate_plus_reporting_bytes=raw + dictionary_bytes + chain_bytes + row_bytes)
    return result


if __name__ == "__main__":
    p = argparse.ArgumentParser()
    p.add_argument("root", nargs="?", type=pathlib.Path)
    p.add_argument("--binary", type=pathlib.Path, default=pathlib.Path("/tmp/klin490-opt-reference/dup-speed"))
    p.add_argument("--indexes", nargs="*", type=pathlib.Path, default=[])
    args = p.parse_args()
    check()
    print(json.dumps(measure(args.root, args.binary, args.indexes) if args.root else {"checks": "passed"}, indent=2))
