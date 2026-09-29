import fs from "node:fs";
import path from "node:path";
import { execFileSync, spawnSync } from "node:child_process";
import { sha256 } from "./trees.ts";

const CUTOFF = "2026-09-29T00:00:00Z";
const CHANGES = 10;
const PER_LANGUAGE = 5;
const LIMIT_MS = 600_000;
const TIP = "refs/replay/tip";

const LANGUAGES = [
  { name: "Rust", manifest: "Cargo.toml" },
  { name: "TypeScript", manifest: "package.json" },
] as const;

function query(language: string): string {
  return `language:${language} stars:1000..20000 pushed:>=2026-09-15 archived:false mirror:false size:<=150000`;
}

interface Change {
  head: string;
  base: string;
  subject: string;
}

interface Repository {
  language: string;
  fullName: string;
  defaultBranch: string;
  stars: number;
  sizeKb: number;
  start: string;
  changes: Change[];
}

interface Selection {
  cutoff: string;
  queries: Record<string, string>;
  repositories: Repository[];
  skipped: { language: string; fullName: string; reason: string }[];
}

interface RunRecord {
  language: string;
  repository: string;
  index: number;
  head: string;
  base: string;
  exit: number | null;
  signal: string | null;
  ms: number;
  report: unknown;
  stdout: string | null;
  stderr: string;
}

interface Commit {
  sha: string;
  time: number;
  subject: string;
}

interface SearchItem {
  full_name: string;
  default_branch: string;
  stargazers_count: number;
  size: number;
}

function git(cwd: string, args: string[]): string {
  return execFileSync("git", args, { cwd, encoding: "utf8", maxBuffer: 1 << 30 }).trimEnd();
}

function holds(cwd: string, commit: string, file: string): boolean {
  return spawnSync("git", ["cat-file", "-e", `${commit}:${file}`], { cwd }).status === 0;
}

function slug(fullName: string): string {
  return fullName.replace("/", "__");
}

function clone(clones: string, item: SearchItem): string | null {
  const into = path.join(clones, slug(item.full_name));
  if (fs.existsSync(path.join(into, ".git"))) {
    return into;
  }
  const url = `https://github.com/${item.full_name}.git`;
  const done = spawnSync("git", ["clone", "--quiet", "--no-tags", "--single-branch", "--branch", item.default_branch, url, into], {
    stdio: "inherit",
  });
  if (done.status !== 0) {
    fs.rmSync(into, { recursive: true, force: true });
    return null;
  }
  git(into, ["update-ref", TIP, "HEAD"]);
  return into;
}

export function changesBefore(walk: Commit[], cutoff: number): Change[] | null {
  const start = walk.findIndex((one) => one.time < cutoff);
  if (start < 0 || walk.length < start + CHANGES + 1) {
    return null;
  }
  return walk.slice(start, start + CHANGES).map((one, at) => ({ head: one.sha, base: walk[start + at + 1].sha, subject: one.subject }));
}

function firstParentWalk(cwd: string): Commit[] {
  return git(cwd, ["log", "--first-parent", "--format=%H%x09%ct%x09%s", TIP])
    .split("\n")
    .map((line) => {
      const [sha, time, ...subject] = line.split("\t");
      return { sha, time: Number(time) * 1000, subject: subject.join("\t") };
    });
}

function eligible(clones: string, language: (typeof LANGUAGES)[number], item: SearchItem): Repository | string {
  const cwd = clone(clones, item);
  if (cwd === null) {
    return "the clone did not finish";
  }
  const changes = changesBefore(firstParentWalk(cwd), Date.parse(CUTOFF));
  if (changes === null) {
    return "the first-parent walk does not reach ten changes before the cutoff";
  }
  const start = changes[0].head;
  if (!holds(cwd, start, language.manifest)) {
    return `the start commit holds no ${language.manifest} at the root`;
  }
  if (holds(cwd, start, "klin.json")) {
    return "the start commit holds a klin.json at the root";
  }
  return {
    language: language.name,
    fullName: item.full_name,
    defaultBranch: item.default_branch,
    stars: item.stargazers_count,
    sizeKb: item.size,
    start,
    changes,
  };
}

function search(into: string, language: string): SearchItem[] {
  const file = path.join(into, `search-${language.toLowerCase()}.json`);
  if (!fs.existsSync(file)) {
    const args = ["api", "-X", "GET", "search/repositories", "-f", `q=${query(language)}`, "-f", "sort=stars", "-f", "order=desc", "-f", "per_page=30"];
    fs.writeFileSync(file, execFileSync("gh", args, { encoding: "utf8" }));
  }
  return (JSON.parse(fs.readFileSync(file, "utf8")) as { items: SearchItem[] }).items;
}

export function select(into: string, clones: string): number {
  fs.mkdirSync(into, { recursive: true });
  fs.mkdirSync(clones, { recursive: true });
  const selection: Selection = { cutoff: CUTOFF, queries: {}, repositories: [], skipped: [] };
  for (const language of LANGUAGES) {
    selection.queries[language.name] = query(language.name);
    let kept = 0;
    for (const item of search(into, language.name)) {
      if (kept === PER_LANGUAGE) {
        break;
      }
      const picked = eligible(clones, language, item);
      if (typeof picked === "string") {
        selection.skipped.push({ language: language.name, fullName: item.full_name, reason: picked });
      } else {
        selection.repositories.push(picked);
        kept += 1;
      }
    }
  }
  fs.writeFileSync(path.join(into, "selection.json"), JSON.stringify(selection, null, 2) + "\n");
  process.stdout.write(
    `selected ${selection.repositories.length} repositories and skipped ${selection.skipped.length}: ` +
      selection.repositories.map((one) => one.fullName).join(", ") + "\n",
  );
  return selection.repositories.length === PER_LANGUAGE * LANGUAGES.length ? 0 : 1;
}

function prepare(cwd: string, change: Change): void {
  spawnSync("git", ["remote", "remove", "origin"], { cwd });
  git(cwd, ["checkout", "--quiet", "--force", "--detach", change.head]);
  for (const branch of git(cwd, ["for-each-ref", "--format=%(refname:short)", "refs/heads"]).split("\n").filter(Boolean)) {
    git(cwd, ["branch", "--quiet", "-D", branch]);
  }
  git(cwd, ["branch", "--quiet", "main", change.base]);
  git(cwd, ["checkout", "--quiet", "-b", "change"]);
  git(cwd, ["reset", "--quiet", "--hard"]);
  git(cwd, ["clean", "--quiet", "-ffdx"]);
  fs.rmSync(path.join(cwd, ".git", "klin"), { recursive: true, force: true });
  fs.writeFileSync(path.join(cwd, "klin.json"), "{}\n");
}

function environment(): NodeJS.ProcessEnv {
  return Object.fromEntries(
    Object.entries(process.env).filter(([name]) => name !== "GITHUB_BASE_REF" && name !== "GITHUB_EVENT_PATH" && !name.startsWith("KLIN_")),
  );
}

function recordPath(into: string, repository: string, index: number, head: string): string {
  return path.join(into, "runs", slug(repository), `${String(index + 1).padStart(2, "0")}-${head.slice(0, 12)}.json`);
}

function readSelection(into: string): Selection {
  return JSON.parse(fs.readFileSync(path.join(into, "selection.json"), "utf8")) as Selection;
}

export function run(into: string, clones: string, klin: string): number {
  const provenance = `${klin}.provenance`;
  if (!fs.existsSync(provenance)) {
    process.stdout.write(`${klin} has no provenance file; build it with benchmark/build-klin\n`);
    return 2;
  }
  const recorded = path.join(into, "klin.provenance.json");
  if (fs.existsSync(recorded) && fs.readFileSync(recorded, "utf8") !== fs.readFileSync(provenance, "utf8")) {
    process.stdout.write(`${klin} is not the binary that ${recorded} names; a resumed replay keeps one binary\n`);
    return 2;
  }
  fs.copyFileSync(provenance, recorded);
  const selection = readSelection(into);
  for (const repository of selection.repositories) {
    const cwd = path.join(clones, slug(repository.fullName));
    repository.changes.forEach((change, index) => {
      const out = recordPath(into, repository.fullName, index, change.head);
      if (fs.existsSync(out)) {
        return;
      }
      prepare(cwd, change);
      const started = Date.now();
      const done = spawnSync(klin, ["gate", "--json"], {
        cwd,
        env: environment(),
        encoding: "utf8",
        timeout: LIMIT_MS,
        maxBuffer: 1 << 30,
      });
      const ms = Date.now() - started;
      let report: unknown = null;
      try {
        report = JSON.parse(done.stdout);
      } catch {
        report = null;
      }
      const record: RunRecord = {
        language: repository.language,
        repository: repository.fullName,
        index,
        head: change.head,
        base: change.base,
        exit: done.status,
        signal: done.signal,
        ms,
        report,
        stdout: report === null ? done.stdout : null,
        stderr: done.stderr,
      };
      fs.mkdirSync(path.dirname(out), { recursive: true });
      fs.writeFileSync(out, JSON.stringify(record, null, 2) + "\n");
      process.stdout.write(`${repository.fullName} ${index + 1}/${repository.changes.length} ${change.head.slice(0, 12)} exit ${done.status} in ${ms} ms\n`);
    });
  }
  return 0;
}

const FLOOR: Record<string, number> = { cc: 5, lines: 25 };
const EXCERPT_LINES = 40;
const EXCERPTS = 3;
const PROMPT_CHARS = 600;
const MESSAGE_LINES = 30;

interface Finding {
  gate?: string;
  file?: string | null;
  line?: number;
  text?: string;
  outcome?: string;
  condition?: string;
  values?: Record<string, unknown>;
  ceiling?: string | null;
  matched?: { file: string; line: number; text: string; values: Record<string, unknown> } | null;
  fix_advice?: string;
}

interface Derived {
  section: string;
  key: string | null;
  value: unknown;
  rule: string;
}

interface Report {
  status?: string;
  window?: Record<string, unknown>;
  derived?: Derived[];
  gates?: { name: string; status: string }[];
  findings?: Finding[];
}

interface Row {
  id: string;
  gate: string;
  status: string;
  groups: string[];
  derived: Derived[];
  findings: (Finding & { excerpt?: string })[];
  remedies: string[];
  context: Record<string, unknown>;
}

function derivedFor(gate: string, derived: Derived[]): Derived[] {
  return derived.filter((one) => one.section === gate.replaceAll("-", "_"));
}

function ceilingOf(finding: Finding, measure: string): number | null {
  const found = new RegExp(`${measure} (\\d+)`).exec(finding.ceiling ?? "");
  return found ? Number(found[1]) : null;
}

export function groupOf(gate: string, finding: Finding, derived: Derived[]): string[] {
  if (gate === "doc-size") {
    return [`document ${finding.file}`];
  }
  if (gate !== "complexity") {
    return [gate];
  }
  if (finding.outcome !== "new" && finding.outcome !== "worsened") {
    return [`${finding.outcome ?? "no"} record`];
  }
  if (!derived.some((one) => one.section === "complexity" && (one.key === "cc" || one.key === "lines"))) {
    return ["pinned ceiling"];
  }
  const over = Object.keys(FLOOR).filter((measure) => {
    const ceiling = ceilingOf(finding, measure);
    return ceiling !== null && Number(finding.values?.[measure]) > ceiling;
  });
  if (over.length === 0) {
    return ["no measure over its ceiling"];
  }
  return over.map((measure) => {
    const ceiling = ceilingOf(finding, measure);
    const source = ceiling === FLOOR[measure] ? "at the floor" : "at a derived percentile";
    const held = Number(finding.matched?.values?.[measure]) > (ceiling ?? Infinity) ? ", the base site already over" : "";
    return `${measure} ${source}${held}`;
  });
}

function clipped(message: string): string {
  const lines = message.split("\n");
  return lines.length <= MESSAGE_LINES ? message : [...lines.slice(0, MESSAGE_LINES), `(${lines.length - MESSAGE_LINES} more lines)`].join("\n");
}

function show(cwd: string, commit: string, file: string): string[] | null {
  const done = spawnSync("git", ["show", `${commit}:${file}`], { cwd, encoding: "utf8", maxBuffer: 1 << 30 });
  return done.status === 0 ? done.stdout.split("\n") : null;
}

function excerpt(cwd: string, commit: string, gate: string, finding: Finding): string | undefined {
  if (!finding.file || !finding.line || gate === "doc-size") {
    return undefined;
  }
  const lines = show(cwd, commit, finding.file);
  if (lines === null) {
    return undefined;
  }
  const span = gate === "complexity" ? Math.min(Number(finding.values?.lines ?? 1), EXCERPT_LINES) : 4;
  const from = Math.max(1, finding.line - 3);
  const to = Math.min(lines.length, finding.line + span);
  const width = String(to).length;
  return lines
    .slice(from - 1, to)
    .map((text, at) => `${String(from + at).padStart(width, "0")} | ${text}`)
    .join("\n");
}

function gateRows(report: Report, context: Record<string, unknown>, detail: (gate: string, finding: Finding) => string | undefined): Omit<Row, "id">[] {
  const derived = report.derived ?? [];
  const findings = report.findings ?? [];
  const failed = (report.gates ?? []).filter((gate) => gate.status !== "ok");
  const rows = failed.map((gate) => {
    const own = findings.filter((finding) => finding.gate === gate.name);
    return {
      gate: gate.name,
      status: gate.status,
      groups: [...new Set(own.flatMap((finding) => groupOf(gate.name, finding, derived)))].sort(),
      derived: derivedFor(gate.name, derived),
      findings: own.map(({ fix_advice: _advice, ...finding }, at) => ({ ...finding, excerpt: at < EXCERPTS ? detail(gate.name, finding) : undefined })),
      remedies: [...new Set(own.map((finding) => finding.fix_advice).filter((one): one is string => Boolean(one)))],
      context: { ...context },
    };
  });
  const loose = findings.filter((finding) => !finding.gate);
  if (rows.length === 0 && (loose.length > 0 || (report.status ?? "PASS") !== "PASS")) {
    rows.push({ gate: "run", status: report.status ?? "none", groups: ["run"], derived: [], findings: loose, remedies: [], context });
  }
  return rows;
}

function numbered(prefix: string, rows: Omit<Row, "id">[]): Row[] {
  return rows.map((row, at) => ({ id: `${prefix}${String(at + 1).padStart(3, "0")}`, ...row }));
}

function replayRows(into: string, clones: string): Row[] {
  const rows: Omit<Row, "id">[] = [];
  for (const repository of readSelection(into).repositories) {
    const cwd = path.join(clones, slug(repository.fullName));
    repository.changes.forEach((change, index) => {
      const record = JSON.parse(fs.readFileSync(recordPath(into, repository.fullName, index, change.head), "utf8")) as RunRecord;
      const context = {
        repository: repository.fullName,
        language: repository.language,
        change: index + 1,
        head: change.head,
        base: change.base,
        exit: record.exit,
        message: clipped(git(cwd, ["log", "-1", "--format=%B", change.head])),
        touched: git(cwd, ["diff", "--stat=120", "--stat-count=30", change.base, change.head]),
      };
      if (record.report === null) {
        rows.push({
          gate: "run",
          status: record.signal ? `killed by ${record.signal}` : `exit ${record.exit}`,
          groups: ["run"],
          derived: [],
          findings: [],
          remedies: [],
          context: { ...context, stdout: (record.stdout ?? "").slice(0, 2000), stderr: record.stderr.slice(0, 2000) },
        });
        return;
      }
      rows.push(...gateRows(record.report as Report, context, (gate, finding) => excerpt(cwd, change.head, gate, finding)));
    });
  }
  return numbered("R", rows);
}

function identity(findings: Finding[]): string {
  return JSON.stringify(findings.map(({ file, line, text, outcome, values }) => [file, line, text, outcome, values]));
}

export function journalRows(lines: string[]): Row[] {
  const rows: (Omit<Row, "id"> & { key: string })[] = [];
  const prompts = new Map<string, string>();
  const firstPrompts = new Map<string, string>();
  const open = new Map<string, Map<string, Omit<Row, "id"> & { key: string }>>();
  for (const line of lines.filter(Boolean)) {
    const record = JSON.parse(line) as Report & { kind: string; session: string | null; text?: string; time: number; version: string; host?: string };
    const session = record.session ?? "unknown";
    if (record.kind === "prompt" && record.text) {
      prompts.set(session, record.text);
      if (!firstPrompts.has(session)) firstPrompts.set(session, record.text);
    }
    if (record.kind !== "stop") {
      continue;
    }
    const previous = open.get(session) ?? new Map();
    const next = new Map<string, Omit<Row, "id"> & { key: string }>();
    if (record.status === "FAIL" || record.status === "ERROR") {
      const window = record.window ?? {};
      const context = {
        session: session.slice(0, 8),
        date: new Date(record.time * 1000).toISOString().slice(0, 10),
        version: record.version,
        host: record.host ?? null,
        window: `${window.kind}, before ${String(window.before ?? "").slice(0, 12)}, ${window.how}`,
        firstPrompt: (firstPrompts.get(session) ?? "").slice(0, PROMPT_CHARS),
        prompt: (prompts.get(session) ?? "").slice(0, PROMPT_CHARS),
        stops: 1,
      };
      for (const row of gateRows(record, context, () => undefined)) {
        const key = row.gate + row.status + identity(row.findings);
        const same = previous.get(row.gate);
        if (same && same.key === key) {
          (same.context as { stops: number }).stops += 1;
          next.set(row.gate, same);
        } else {
          const fresh = { ...row, key };
          rows.push(fresh);
          next.set(row.gate, fresh);
        }
      }
    }
    open.set(session, next);
  }
  return numbered("J", rows.map(({ key: _key, ...row }) => row));
}

function quote(text: string): string {
  return text
    .trimEnd()
    .split("\n")
    .map((line) => `> ${line}`.trimEnd())
    .join("\n");
}

function fence(text: string): string {
  return "```text\n" + text.replaceAll("```", "` ` `") + "\n```";
}

function findingLine(finding: Finding): string {
  const site = finding.file ? `\`${finding.file}${finding.line ? `:${finding.line}` : ""}\`` : "no file";
  const parts = [`${site} ${finding.outcome ?? ""}`.trim()];
  if (finding.text) parts.push(`\`${finding.text.replaceAll("`", "'").replaceAll("\n", " ").slice(0, 160)}\``);
  if (finding.values) parts.push(`values \`${JSON.stringify(finding.values)}\``);
  if (finding.ceiling) parts.push(`ceiling ${finding.ceiling}`);
  if (finding.matched) parts.push(`base site \`${finding.matched.file}:${finding.matched.line}\` with \`${JSON.stringify(finding.matched.values)}\``);
  else if (finding.outcome === "new") parts.push("nothing at the base matched");
  return parts.join(", ");
}

function rowMarkdown(row: Row): string {
  const context = row.context;
  const out = [`## ${row.id}`, ""];
  if (context.repository) {
    out.push(`- Repository: \`${context.repository}\` (${context.language}), change ${context.change} of ${CHANGES}`);
    out.push(`- Commit: \`${String(context.head).slice(0, 12)}\`, judged against its first parent \`${String(context.base).slice(0, 12)}\`, exit ${context.exit}`);
  } else {
    out.push(`- Session \`${context.session}\` on ${context.date}, klin ${context.version}, host ${context.host ?? "unknown"}`);
    out.push(`- Window: ${context.window}`);
    out.push(`- Stops this row stands for: ${context.stops}`);
  }
  out.push(`- Gate: ${row.gate}, ${row.status}`);
  out.push(`- Decision group: ${row.groups.join("; ") || "none"}`);
  for (const one of row.derived) {
    out.push(`- Derived ${one.key ?? one.section}: \`${JSON.stringify(one.value)}\`, ${one.rule}`);
  }
  out.push("");
  if (context.message !== undefined) {
    out.push("### Commit message", "", quote(String(context.message)), "", "### Files the change touched", "", fence(String(context.touched)), "");
  }
  if (context.firstPrompt !== undefined && context.firstPrompt !== context.prompt) {
    out.push("### First prompt of the session", "", context.firstPrompt ? quote(String(context.firstPrompt)) : "No prompt text was recorded.", "");
  }
  if (context.prompt !== undefined) {
    out.push("### Last prompt of the session before the stop", "", context.prompt ? quote(String(context.prompt)) : "No prompt text was recorded.", "");
  }
  if (context.stderr !== undefined) {
    out.push("### What the run printed", "", fence(`${context.stdout}\n${context.stderr}`.trim()), "");
  }
  if (row.findings.length > 0) {
    const conditions = [...new Set(row.findings.map((finding) => finding.condition).filter(Boolean))];
    out.push("### Findings", "", ...conditions.map((one) => `Condition: ${one}.`), "");
    row.findings.forEach((finding, at) => {
      out.push(`${at + 1}. ${findingLine(finding)}`);
      if (finding.excerpt) out.push("", fence(finding.excerpt).replace(/^/gm, "   "), "");
    });
    out.push("");
  }
  if (row.remedies.length > 0) {
    out.push("### Remedy klin printed", "", ...row.remedies.map((one) => quote(one) + "\n"));
  }
  return out.join("\n").trimEnd() + "\n";
}

function worksheetMarkdown(title: string, preface: string[], rows: Row[]): string {
  const byGate = new Map<string, number>();
  for (const row of rows) byGate.set(row.gate, (byGate.get(row.gate) ?? 0) + 1);
  const head = [
    `# ${title}`,
    "",
    ...preface,
    "",
    "Label each row `appropriate` or `not-appropriate` in `labels.json`, with an optional note. Judge the row as it stood when it fired.",
    "",
    `- Rows: ${rows.length}`,
    ...[...byGate].sort().map(([gate, count]) => `- ${gate}: ${count}`),
    "",
  ];
  return head.join("\n") + rows.map(rowMarkdown).join("\n");
}

export function worksheets(into: string, clones: string, journal: string): number {
  const replay = replayRows(into, clones);
  const text = fs.readFileSync(journal, "utf8");
  const lines = text.split("\n");
  const stops = lines.filter(Boolean).map((line) => JSON.parse(line) as { kind: string; status?: string });
  const failing = stops.filter((one) => one.kind === "stop" && (one.status === "FAIL" || one.status === "ERROR")).length;
  const own = journalRows(lines);
  const runs = readSelection(into).repositories.reduce((sum, one) => sum + one.changes.length, 0);
  fs.writeFileSync(path.join(into, "worksheet.json"), JSON.stringify(replay, null, 2) + "\n");
  fs.writeFileSync(
    path.join(into, "worksheet.md"),
    worksheetMarkdown("Replay worksheet", [`Every gate that failed or erred in the ${runs} replay runs of \`klin gate --json\` with \`{}\`, one row each.`], replay),
  );
  fs.writeFileSync(path.join(into, "journal-worksheet.json"), JSON.stringify(own, null, 2) + "\n");
  fs.writeFileSync(
    path.join(into, "journal-worksheet.md"),
    worksheetMarkdown(
      "Journal worksheet",
      [
        `Every gate that failed or erred in the ${failing} FAIL or ERROR stops of klin's own journal, SHA-256 \`${sha256(text)}\`.`,
        "Consecutive stops of one session with the same findings for a gate share a row.",
      ],
      own,
    ),
  );
  const labels = Object.fromEntries([...replay, ...own].map((row) => [row.id, { label: null, note: "" }]));
  const file = path.join(into, "labels.json");
  if (!fs.existsSync(file)) {
    fs.writeFileSync(file, JSON.stringify(labels, null, 2) + "\n");
  }
  process.stdout.write(`wrote ${replay.length} replay rows and ${own.length} journal rows over ${failing} failing stops\n`);
  return 0;
}
