# Turns the `klin check --json` document into the Action's annotations (`$part ==
# "annotations"`) or its job summary (`$part == "summary"`). Spec 12.3.

def per_level: 10;

def failing: [.findings[] | select(.outcome != "held")];

def flat: tostring | gsub("[\r\n]+"; " ");

def command_data: tostring | gsub("%"; "%25") | gsub("\r"; "%0D") | gsub("\n"; "%0A");

def command_property: command_data | gsub(":"; "%3A") | gsub(","; "%2C");

def line_of: .line // .values.line?;

def site: .file + (if line_of then ":\(line_of)" else "" end);

def values_said: .values | to_entries | map("\(.key) \(.value)") | join(", ");

def finding_said:
  if .kind == "measurement-lost" then "\(.condition) \(.remedy)"
  else "\(.outcome) \(.check) finding (\(values_said)), \(.condition). \(.remedy)"
  end;

def review_said: .text + (if .reason then " (\(.reason))" else "" end);

def annotation(level; title; said):
  "::\(level) file=\(.file | command_property)"
  + (if line_of then ",line=\(line_of)" else "" end)
  + ",title=\(title | command_property)::\(said | command_data)";

def annotations:
  (failing[:per_level][] | annotation("error"; "klin: \(.check // .kind)"; finding_said)),
  (.reviews[:per_level][] | annotation("warning"; "klin review: \(.kind)"; review_said));

def summary_budget: 900000;

def section(heading; items):
  if (items | length) == 0 then empty
  else "", "### \(heading) (\(items | length))", "", (items[] | "- \(flat)")
  end;

def holes: [.measurements[] | .check as $check | .holes[]
  | "\($check // "run"): \(.reason)" + (if .detail then " (\(.detail))" else "" end) + " — \(.text)"];

def capped(budget):
  reduce .[] as $line ({lines: [], bytes: 0, left: 0};
    ($line | utf8bytelength + 1) as $size
    | if .left == 0 and .bytes + $size <= budget then .lines += [$line] | .bytes += $size
      else .left += 1 end)
  | .lines[], (if .left > 0 then "", "The summary stops here to stay under GitHub's step summary limit: \(.left) line(s) left out. Run `klin check` for the full report." else empty end);

def summary:
  "## klin check",
  "",
  "judgement: \(.judgement // "none"), measurement: \(.measurement // "none"), execution: \(.execution), exit \(.exit)",
  "",
  "failing findings: \(failing | length), review items: \(.reviews | length), holes: \(holes | length), errors: \(.errors | length), files not measured: \(.not_measured)",
  "",
  "Not annotated: \([(failing | length) - per_level, 0] | max) failing finding(s) and \([(.reviews | length) - per_level, 0] | max) review item(s) over GitHub's limit of \(per_level) annotations per level, and every hole and error. The lists below hold them, up to the summary's size budget.",
  ([section("Errors"; [.errors[] | "\(.check // "run") \(.kind): \(.message)"]),
    section("Holes"; holes),
    section("Failing findings"; [failing[] | "`\(site)` \(.check // .kind): \(finding_said)"]),
    section("Review items"; [.reviews[] | "`\(site)` \(.kind): \(review_said)"])] | capped(summary_budget));

if $part == "annotations" then annotations else summary end
