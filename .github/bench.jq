# The deterministic counters of one `klin gate --strict --json` report, as <gate>.<field>.
# Every time and memory value is left out: a shared runner is not a timing oracle (ADR 0042).
# With no `$base` the program prints those counters. With one it prints the Markdown comparison.
def rise: 10;

def flat:
  { findings: (.findings | length) }
  + ( [ .gates[]
        | . as $gate
        | ($gate | paths(type == "number")) as $path
        | { key: ([$gate.name] + $path | join(".")), value: ($gate | getpath($path)) } ]
      | from_entries )
  | with_entries(select(.key | test("(_|\\.)ms$|rss") | not));

def shown($held): if $held == null then "—" else ($held | tostring) end;

def percent($before; $after):
  if $before == null or $after == null then null
  elif $before == 0 then (if $after > 0 then infinite else 0 end)
  else (($after - $before) / $before * 1000 | round / 10)
  end;

def told($percent):
  if $percent == null then "—"
  elif $percent == infinite then "new"
  else (if $percent > 0 then "+" else "" end) + ($percent | tostring) + "%"
  end;

def line($key; $before; $after):
  percent($before; $after) as $percent
  | (if $after == null then "gone" else told($percent) end) as $change
  | (if $after == null or ($percent != null and $percent > rise) then " ⚠" else "" end) as $mark
  | "| `\($key)`\($mark) | \(shown($before)) | \(shown($after)) | \($change) |";

flat as $head
| if ($base | length) == 0 then $head
  else
    ($base + $head | keys) as $every
    | ($every | map(select($base[.] != $head[.]))) as $moved
    | [ "<!-- klin-benchmark -->",
        "## Benchmark: the base binary against this branch's binary",
        "",
        "Both binaries ran `gate --strict` over the same tree at the same commit, so only the",
        "binary differs." ]
      + ( if ($moved | length) == 0
          then [ "", "All \($every | length) counters agree." ]
          else [ "",
                 "| Measurement | Base | This branch | Change |",
                 "| --- | ---: | ---: | ---: |" ]
               + ($moved | map(line(.; $base[.]; $head[.])))
               + [ "",
                   "\($moved | length) of \($every | length) counters moved. ⚠ marks a counter",
                   "that rose by more than \(rise)%, a counter that rose from zero, or a measurement",
                   "the base reported and this branch does not." ]
          end )
      + [ "",
          "Times and peak RSS are left out, because a shared runner is not a timing oracle",
          "(ADR 0042)." ]
    | .[]
  end
