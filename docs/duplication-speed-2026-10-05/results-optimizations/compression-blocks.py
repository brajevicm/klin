"""Standard block compression of exact O5 artifacts; byte roundtrip and access accounting."""
import argparse
import hashlib
import json
import sys
# Python 3.14 lzma imports stdlib compression._common; sibling compression.py
# would shadow that package during the import. Load our sibling explicitly below.
script_dir = sys.path.pop(0)
import lzma
sys.path.insert(0, script_dir)
import pathlib
import struct
import subprocess
import zlib
import time
import resource
import tempfile
import importlib.util
spec = importlib.util.spec_from_file_location("o5_base", pathlib.Path(__file__).with_name("compression.py"))
base = importlib.util.module_from_spec(spec)
spec.loader.exec_module(base)

BLOCK = 65536
# Each directory record has uncompressed start, stored offset, raw length, stored length.
RECORD = struct.Struct('<QQII')


def compress_blocks(data, codec):
    compress, decompress = (zlib.compress, zlib.decompress) if codec == 'zlib' else (lzma.compress, lzma.decompress)
    payload = bytearray()
    directory = bytearray()
    for start in range(0, len(data), BLOCK):
        block = data[start:start + BLOCK]
        compressed = compress(block)
        # A one-byte selector permits raw storage for incompressible blocks.
        stored = b'\1' + compressed if len(compressed) < len(block) else b'\0' + block
        directory.extend(RECORD.pack(start, len(payload), len(block), len(stored)))
        payload.extend(stored)
    restored = bytearray()
    for start, offset, raw_len, stored_len in RECORD.iter_unpack(directory):
        stored = payload[offset:offset + stored_len]
        block = decompress(stored[1:]) if stored[0] else stored[1:]
        assert start == len(restored) and len(block) == raw_len
        restored.extend(block)
    assert restored == data
    return dict(payload_bytes=len(payload), block_directory_bytes=len(directory),
                total_bytes=len(payload) + len(directory), blocks=len(directory) // RECORD.size)


def measure(root, binary, indexes, return_components=False):
    dictionary, inverse = {}, [None]
    chains, rows, paths, file_directory = bytearray(), bytearray(), bytearray(), bytearray()
    sparse = bytearray()
    files = tokens = unsafe = 0
    digest = hashlib.sha256()
    for path in sorted(root.rglob('*')):
        relative = path.relative_to(root)
        if not path.is_file() or not base.eligible(relative):
            continue
        data = json.loads(subprocess.check_output([str(binary), 'normalize', str(relative)], cwd=root))
        raw = [bytes.fromhex(t) for t in data['tokens']]
        chain = base.encode(raw, data['safe'], dictionary, inverse)
        assert base.decode(chain, dictionary, inverse) == [t if ok else None for t, ok in zip(raw, data['safe'])]
        row_data = b''.join(base.varint(row - (data['rows'][i-1] if i else 0)) for i, row in enumerate(data['rows']))
        reconstructed_rows, current = [], 0
        for delta in base.numbers(row_data):
            current += delta
            reconstructed_rows.append(current)
        assert reconstructed_rows == data['rows']
        chain_offset, row_offset = len(chains), len(rows)
        for i, (token_id, delta) in enumerate(zip(base.numbers(chain), base.numbers(row_data))):
            if i % 256 == 0:
                sparse.extend(struct.pack('<QQQQQ', files, i, chain_offset, row_offset, data['rows'][i-1] if i else 0))
            chain_offset += len(base.varint(token_id))
            row_offset += len(base.varint(delta))
        # One fixed file record: path offset, chain offset, row offset, token count.
        file_directory.extend(struct.pack('<QQQQ', len(paths), len(chains), len(rows), len(raw)))
        name = str(relative).encode()
        paths.extend(base.varint(len(name)) + name)
        chains.extend(chain)
        rows.extend(row_data)
        files += 1
        tokens += len(raw)
        unsafe += sum(not ok for ok in data['safe'])
    dictionary_data = base.varint(len(dictionary)) + b''.join(base.varint(len(t)) + t for t in dictionary)
    # Dictionary ID access uses eight-byte raw offsets. Block records map raw offsets to stored bytes.
    dictionary_offsets = bytearray()
    offset = len(base.varint(len(dictionary)))
    for token in dictionary:
        dictionary_offsets.extend(struct.pack('<Q', offset))
        offset += len(base.varint(len(token))) + len(token)
    components = dict(dictionary=dictionary_data, dictionary_offsets=dictionary_offsets,
                      chains=chains, rows=rows, paths=paths, files=file_directory, sparse_token_offsets=sparse)
    for name, data in components.items():
        digest.update(name.encode() + struct.pack('<Q', len(data)) + data)
    result = dict(root=str(root), files=files, tokens=tokens, unsafe_tokens=unsafe,
                  normalizer_sha256=hashlib.sha256(binary.read_bytes()).hexdigest(),
                  exact_artifact_sha256=digest.hexdigest(), block_bytes=BLOCK,
                  raw_component_bytes={name: len(data) for name, data in components.items()},
                  roundtrip_all_bytes=True, lineage_bytes=None, timing_claim=False, codecs={})
    if return_components:
        return result, components
    for codec in ('zlib', 'lzma'):
        sizes = {name: compress_blocks(data, codec) for name, data in components.items()}
        total = sum(item['total_bytes'] for item in sizes.values())
        # Header stores version, block size, component stored offsets and lengths.
        header = 16 + len(components) * 16
        totals = {}
        for index in indexes:
            candidate = (index / 'index.bin').stat().st_size
            # Candidate already includes paths; retain compressed path metadata too because
            # this experiment's independent file directory requires it. No optimistic subtraction.
            totals[index.name] = candidate + total + header
        result['codecs'][codec] = dict(components=sizes, header_bytes=header,
            complete_evidence_bytes=total + header, candidate_plus_evidence_bytes=totals)
    return result


def candidate_measure(indexes, evidence):
    result = dict(roundtrip_all_bytes=True, indexed_lookup_implemented=False,
                  timing_claim=False, lineage_bytes=None, candidates={})
    for index in indexes:
        raw = (index / 'index.bin').read_bytes()
        item = dict(raw_bytes=len(raw), sha256=hashlib.sha256(raw).hexdigest(), codecs={})
        for codec in ('zlib', 'lzma'):
            sizes = compress_blocks(raw, codec)
            # Standalone candidate container: version/block size + component offset/length.
            sizes['header_bytes'] = 32
            sizes['complete_candidate_bytes'] = sizes['total_bytes'] + 32
            if evidence:
                sizes['candidate_plus_evidence_bytes'] = (sizes['complete_candidate_bytes']
                    + evidence['codecs'][codec]['complete_evidence_bytes'])
            item['codecs'][codec] = sizes
        result['candidates'][index.name] = item
    return result


def profile_worker(directory, codec):
    components = {path.stem: path.read_bytes() for path in sorted(directory.glob('*.raw'))}
    compress = zlib.compress if codec == 'zlib' else lambda data: lzma.compress(data, preset=int(codec[-1]))
    decompress = zlib.decompress if codec == 'zlib' else lzma.decompress
    rows = []
    for iteration in range(5):
        start = time.perf_counter_ns()
        encoded = {}
        for name, data in components.items():
            payload, records = bytearray(), bytearray()
            for offset in range(0, len(data), BLOCK):
                raw = data[offset:offset + BLOCK]
                packed = compress(raw)
                stored = b'\1' + packed if len(packed) < len(raw) else b'\0' + raw
                records.extend(RECORD.pack(offset, len(payload), len(raw), len(stored)))
                payload.extend(stored)
            encoded[name] = (payload, records)
        compression_ns = time.perf_counter_ns() - start
        start = time.perf_counter_ns()
        decoded = {}
        for name, (payload, records) in encoded.items():
            restored = bytearray()
            for offset, stored_offset, raw_len, stored_len in RECORD.iter_unpack(records):
                stored = payload[stored_offset:stored_offset + stored_len]
                block = decompress(stored[1:]) if stored[0] else stored[1:]
                assert offset == len(restored) and len(block) == raw_len
                restored.extend(block)
            decoded[name] = restored
        decompression_ns = time.perf_counter_ns() - start
        assert all(decoded[name] == data for name, data in components.items())
        sizes = {name: len(payload) + len(records) for name, (payload, records) in encoded.items()}
        # Separate seven-component evidence container and one-component candidate container.
        header_bytes = 16 + (len(components) - 1) * 16 + 32
        rows.append(dict(iteration=iteration, compression_ms=compression_ns / 1e6,
                         decompression_ms=decompression_ns / 1e6,
                         component_stored_bytes=sizes, complete_bytes=sum(sizes.values()) + header_bytes))
        del encoded, decoded
    rss = resource.getrusage(resource.RUSAGE_SELF).ru_maxrss
    return dict(codec=codec, iterations=rows, peak_rss_bytes=rss if sys.platform == 'darwin' else rss * 1024,
                rss_scope='Python worker ru_maxrss: process high-water including raw data, codec buffers, interpreter and possible inherited launch high-water; not isolated codec memory',
                random_access_lookup_implemented=False, all_decoded_bytes_equal=True)


def profile(root, binary, index):
    result, components = measure(root, binary, [], return_components=True)
    reference = json.loads(pathlib.Path(__file__).with_name('compression-blocks-1m.json').read_text())
    assert result['exact_artifact_sha256'] == reference['exact_artifact_sha256']
    candidate = (index / 'index.bin').read_bytes()
    result['candidate_sha256'] = hashlib.sha256(candidate).hexdigest()
    components['candidate'] = candidate
    result['codec_profiles'] = {}
    with tempfile.TemporaryDirectory(prefix='klin-o5-profile-') as folder:
        directory = pathlib.Path(folder)
        for name, data in components.items():
            (directory / (name + '.raw')).write_bytes(data)
        for codec in ('zlib', 'lzma0', 'lzma6'):
            result['codec_profiles'][codec] = json.loads(subprocess.check_output(
                [sys.executable, __file__, '--profile-worker', str(directory), codec]))
    result['timing_claim'] = 'isolated Python codec stage only, excludes normalization and production integration'
    result['lineage_bytes'] = None
    return result


if __name__ == '__main__':
    parser = argparse.ArgumentParser()
    parser.add_argument('root', nargs='?', type=pathlib.Path)
    parser.add_argument('--binary', type=pathlib.Path, default=pathlib.Path('/tmp/klin490-opt-reference/dup-speed'))
    parser.add_argument('--indexes', nargs='*', type=pathlib.Path, default=[])
    parser.add_argument('--evidence-result', type=pathlib.Path)
    parser.add_argument('--profile', action='store_true')
    parser.add_argument('--profile-worker', nargs=2)
    args = parser.parse_args()
    if args.profile_worker:
        print(json.dumps(profile_worker(pathlib.Path(args.profile_worker[0]), args.profile_worker[1])))
        sys.exit(0)
    base.check()
    for codec in ('zlib', 'lzma'):
        for data in (b'', b'x', bytes(range(256)) * 600, b'\0' * 200000):
            compress_blocks(data, codec)
    if args.profile:
        assert args.root and len(args.indexes) == 1
        result = profile(args.root, args.binary, args.indexes[0])
    elif args.root:
        result = measure(args.root, args.binary, args.indexes)
    elif args.indexes:
        evidence = json.loads(args.evidence_result.read_text()) if args.evidence_result else None
        result = candidate_measure(args.indexes, evidence)
    else:
        result = {'checks': 'passed'}
    print(json.dumps(result, indent=2))
