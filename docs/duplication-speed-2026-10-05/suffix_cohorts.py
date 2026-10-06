"""Generalized suffix/LCP exact families without witness-pair enumeration."""
from collections import Counter
import json
import region_cohorts as seeds
import families


def suffix_array(values):
    order = list(range(len(values)))
    ranks = values[:]
    width = 1
    while width < len(values):
        order.sort(key=lambda i: (ranks[i], ranks[i + width] if i + width < len(values) else -10**30))
        updated = [0] * len(values)
        for j in range(1, len(order)):
            a, b = order[j - 1], order[j]
            updated[b] = updated[a] + ((ranks[a], ranks[a + width] if a + width < len(values) else -10**30) != (ranks[b], ranks[b + width] if b + width < len(values) else -10**30))
        ranks = updated
        if not order or ranks[order[-1]] == len(order) - 1:
            break
        width *= 2
    return order


def indexed(records, threshold):
    assert threshold > 0
    grouped = {}
    for name, stream in records:
        grouped.setdefault((tuple(stream['tokens']), tuple(stream['safe'])), []).append(name)
    classes = list(grouped.items())
    values, locations, predecessors = [], [], []
    ids = {}
    output = {}
    boundary = 0
    for class_id, ((tokens, safe), names) in enumerate(classes):
        first = 0
        for end in range(len(tokens) + 1):
            if end < len(tokens) and safe[end]:
                continue
            if len(names) > 1 and end - first >= threshold:
                families.add(output, tokens, names, names, first, first, end)
            if end > first:
                boundary -= 1
                for pos in range(first, end):
                    value = ids.setdefault(tokens[pos], len(ids) + 1)
                    values.append(value)
                    locations.append((class_id, pos))
                    predecessors.append(('boundary', boundary) if pos == first else ('token', ids[tokens[pos - 1]]))
                values.append(boundary)
                locations.append(None)
                predecessors.append(None)
            first = end + 1
    order = suffix_array(values)
    inverse = [0] * len(values)
    for rank, pos in enumerate(order):
        inverse[pos] = rank
    lcp = [0] * len(values)
    height = 0
    for pos in range(len(values)):
        rank = inverse[pos]
        if rank == 0:
            height = 0
            continue
        other = order[rank - 1]
        while pos + height < len(values) and other + height < len(values) and values[pos + height] == values[other + height]:
            height += 1
        lcp[rank] = height
        height = max(0, height - 1)
    stack = []
    intervals = []
    for right in range(1, len(order) + 1):
        depth = lcp[right] if right < len(order) else 0
        left = right - 1
        while stack and stack[-1][0] > depth:
            old_depth, left = stack.pop()
            if old_depth >= threshold:
                intervals.append((old_depth, left, right))
        if depth and (not stack or stack[-1][0] < depth):
            stack.append((depth, left))
    leaf_visits = 0
    for depth, left, right in intervals:
        total_predecessors = Counter(predecessors[order[i]] for i in range(left, right))
        child = left
        for end in range(left + 1, right + 1):
            if end < right and lcp[end] > depth:
                continue
            child_predecessors = Counter(predecessors[order[i]] for i in range(child, end))
            for i in range(child, end):
                leaf_visits += 1
                pos = order[i]
                pred = predecessors[pos]
                partners = (right - left) - (end - child) - total_predecessors[pred] + child_predecessors[pred]
                if partners:
                    class_id, start = locations[pos]
                    (tokens, _), names = classes[class_id]
                    key = tuple(tokens[start:start + depth])
                    output.setdefault(key, set()).update((name, start, start + depth) for name in names)
            child = end
    return output, {'classes': len(classes), 'suffixes': len(values), 'lcp_intervals': len(intervals), 'interval_leaf_visits': leaf_visits, 'witness_pairs_enumerated': 0, 'families': len(output), 'occurrences': sum(map(len, output.values()))}


def main():
    rows = []
    for label, records, threshold in seeds.cases():
        actual, metrics = indexed(records, threshold)
        expected = ({tuple(range(30)): {(str(i), 0, 30) for i in range(1000)}} if label == '1000-identical' else families.exhaustive(records, threshold))
        assert actual == expected, label
        rows.append({'case': label, 'verification': 'exact-expected-mapping' if label == '1000-identical' else 'exhaustive-oracle', **metrics})
    print(json.dumps({'scope': 'suffix/LCP current-content diagnostic; no lineage or production budget claim', 'cases': rows}, indent=2))


if __name__ == '__main__':
    main()
