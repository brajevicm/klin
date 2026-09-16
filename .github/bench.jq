# With no `$base` this prints one measurement's deterministic counters. With one it prints the
# Markdown section named by `$title`, so one program serves both calls of the job.
# Times and memory stay out: a shared runner is not a timing oracle (ADR 0042).
def rise: 10;

# A `klin gate --strict --json` report, or a set of counters a row already printed as key=value.
def flat:
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
  if $before == null or $after == null then null
  elif $before == 0 then (if $after > 0 then infinite else 0 end)
  else (($after - $before) / $before * 1000 | round / 10)
  end;

def percent_text($percent):
  if $percent == null then "—"
  elif $percent == infinite then "new"
  else (if $percent > 0 then "+" else "" end) + ($percent | tostring) + "%"
  end;

# A measurement only one side reports is marked whatever its value: the binary changed what it
# counts, which is what the job exists to show.
def line($key; $before; $after):
  percent($before; $after) as $percent
  | (if $after == null then "gone"
     elif $before == null then "new"
     else percent_text($percent)
     end) as $change
  | (if $before == null or $after == null or ($percent != null and $percent > rise)
     then " ⚠" else "" end) as $mark
  | "| `\($key)`\($mark) | \(cell($before)) | \(cell($after)) | \($change) |";

flat as $head
| if ($base | length) == 0 then $head
  else
    ($base + $head | keys) as $every
    | ($every | map(select($base[.] != $head[.]))) as $moved
    | [ "### \($title)", "" ]
      + ( if ($moved | length) == 0
          then [ "All \($every | length) counters agree.", "" ]
          else [ "| Measurement | Base | This branch | Change |",
                 "| --- | ---: | ---: | ---: |" ]
               + ($moved | map(line(.; $base[.]; $head[.])))
               + [ "",
                   "\($moved | length) of \($every | length) counters moved. ⚠ marks one that rose",
                   "by more than \(rise)%, or one that only the base or only this branch reports.",
                   "" ]
          end )
    | .[]
  end
