# The complete benchmark comment. Times and peak RSS are diagnostics from the shared runner, not
# machine-independent budgets (ADR 0042).
def rise: 10;

# A `klin check --json` report, or a set of counters a performance row already printed as
# key=value. Time and memory keys are deliberately removed from the deterministic comparison.
def counters:
  ( if has("gates")
    then { findings: (.findings | length) }
         + ( [ .gates[]
               | . as $gate
               | ($gate | paths(type == "number")) as $path
               | { key: ([$gate.name] + $path | join(".")), value: ($gate | getpath($path)) } ]
             | from_entries )
    else .
    end )
  | with_entries(select(.key | test("(_|\\.)ms$|rss") | not));

def cell($held): if $held == null then "—" else ($held | tostring) end;

def percent($before; $after):
  if ($before | type) != "number" or ($after | type) != "number" then null
  elif $before == 0 then (if $after > 0 then infinite else 0 end)
  else (($after - $before) / $before * 1000 | round / 10)
  end;

def counter_change($percent):
  if $percent == null then "—"
  elif $percent == infinite then "new"
  else (if $percent > 0 then "+" else "" end) + ($percent | tostring) + "%"
  end;

def metric_change($before; $after):
  percent($before; $after) as $percent
  | if $percent == null then "—"
    elif $percent == infinite then "🆕 new"
    elif $percent > 0 then "📈 +\($percent)%"
    elif $percent < 0 then "📉 \($percent)%"
    else "✅ 0%"
    end;

def moved($before; $after):
  (($before + $after) | keys) as $every
  | $every | map(select($before[.] != $after[.]));

def counter_row($key; $before; $after):
  percent($before; $after) as $percent
  | (if $after == null then "gone"
     elif $before == null then "new"
     else counter_change($percent)
     end) as $change
  | (if $before == null or $after == null or ($percent != null and $percent > rise)
     then " ⚠️" else "" end) as $mark
  | "| `\($key)`\($mark) | \(cell($before)) | \(cell($after)) | \($change) |";

def counter_lines($title; $before; $after):
  moved($before; $after) as $moved
  | (($before + $after) | keys) as $every
  | [ "### \($title)", "" ]
    + ( if ($moved | length) == 0
        then [ "All \($every | length) counters agree. ✅", "" ]
        else [ "| Measurement | Base | This branch | Change |",
               "| --- | ---: | ---: | ---: |" ]
             + ($moved | map(counter_row(.; $before[.]; $after[.])))
             + [ "",
                 "\($moved | length) of \($every | length) counters moved. ⚠️ marks one that rose",
                 "by more than \(rise)%, or one that only the base or only this branch reports.",
                 "" ]
        end );

def metric_cell($value; $unit):
  if ($value | type) == "number" then "\($value) \($unit)"
  elif $value == null then "—"
  else ($value | tostring)
  end;

def metric_row($label; $key; $unit; $before; $after):
  "| \($label) | \(metric_cell($before[$key]; $unit)) | \(metric_cell($after[$key]; $unit)) | \(metric_change($before[$key]; $after[$key])) |";

def metric_table($rows; $before; $after):
  [ "| Workload | Base | This branch | Change |",
    "| --- | ---: | ---: | ---: |" ]
  + ($rows | map(metric_row(.label; .key; .unit; $before; $after)));

def time_rows: [
  { key: "warm20_ms", label: "🔥 Warm hook · 20 changed files", unit: "ms" },
  { key: "warm100_ms", label: "📈 Warm hook · 100 changed files", unit: "ms" },
  { key: "uncached_ms", label: "🧊 Warm hook · no structural cache", unit: "ms" },
  { key: "cold_ms", label: "❄️ Cold survey", unit: "ms" },
  { key: "strict_ms", label: "🧭 Strict run", unit: "ms" }
];

def rss_rows: [
  { key: "warm_hook_peak_rss_kb", label: "🔥 Warm hook", unit: "kB" },
  { key: "warm_hook_without_structural_cache_peak_rss_kb", label: "🧊 Warm hook · no structural cache", unit: "kB" },
  { key: "dead_symbols_changed_peak_rss_kb", label: "🎯 `dead-symbols --changed`", unit: "kB" },
  { key: "strict_peak_rss_kb", label: "🧭 Strict run", unit: "kB" }
];

def details($summary; $body):
  [ "<details>",
    "<summary>\($summary)</summary>",
    "" ] + $body + [ "", "</details>", "" ];

def header:
  [ "<!-- klin-benchmark -->",
    "## ⚡ Benchmark: base vs this branch",
    "",
    "Both binaries ran the same measurements on this runner, so only the binary differs.",
    "",
    "> 📌 Measured `\($merge)`, head `\($head)` merged onto base `\($base)`",
    "> 🔁 A later push leaves this stale; remove and re-add the `benchmark` label to measure again",
    "> 🧪 Same `structural_300k` fixture on `ubuntu-latest`",
    "> ⏱️ Times are 5-run medians · 🧠 RSS is a single-run diagnostic",
    "> ℹ️ Performance values are informational; they do not fail the workflow.",
    "" ];

($strict_base[0] | counters) as $strict_before
| ($strict_head[0] | counters) as $strict_after
| ($dense_base[0] | counters) as $dense_before
| ($dense_head[0] | counters) as $dense_after
| ($dense_base[0]) as $perf_before
| ($dense_head[0]) as $perf_after
| (moved($strict_before; $strict_after) + moved($dense_before; $dense_after)) as $moved
| header
  + [ "### 🧭 At a glance", "",
      "| Signal | Base | This branch | Change |",
      "| --- | ---: | ---: | ---: |",
      metric_row("🔥 Warm hook · 20 changed files"; "warm20_ms"; "ms"; $perf_before; $perf_after),
      metric_row("🧠 Warm hook peak RSS"; "warm_hook_peak_rss_kb"; "kB"; $perf_before; $perf_after),
      "| 🧮 Deterministic counters | — | — | " +
        (if ($moved | length) == 0 then "✅ unchanged" else "⚠️ \($moved | length) moved" end) + " |",
      "" ]
  + details("⏱️ Time breakdown"; metric_table(time_rows; $perf_before; $perf_after))
  + details("🧠 Peak RSS"; metric_table(rss_rows; $perf_before; $perf_after))
  + details("🧮 Deterministic counter comparison";
      counter_lines("klin on klin, strict run"; $strict_before; $strict_after)
      + [ "" ] + counter_lines("Dense 300k fixture"; $dense_before; $dense_after))
  | .[]
