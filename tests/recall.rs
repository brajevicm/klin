mod harness;

use harness::Tree;
use serde_json::Value;

const A_STOP: &str = r#"{"hook_event_name": "Stop", "stop_hook_active": false}"#;

const PACKAGE: &str = r#"{
  "name": "chatbot",
  "private": true,
  "scripts": { "dev": "next dev", "build": "next build", "test": "playwright test" },
  "dependencies": { "next": "15.0.0", "react": "19.0.0", "react-dom": "19.0.0" },
  "devDependencies": { "@playwright/test": "1.48.0", "typescript": "5.6.0" }
}
"#;

const LAYOUT: &str = r#"import type { ReactNode } from "react";

export default function RootLayout({ children }: { children: ReactNode }) {
  return (
    <html lang="en">
      <body>{children}</body>
    </html>
  );
}
"#;

const PAGE: &str = r#"import { Chat } from "../components/chat/chat";
import { Header } from "../components/chat/header";
import { Sidebar } from "../components/chat/sidebar";

export default function Page() {
  return (
    <main className="flex h-dvh">
      <Sidebar />
      <div className="flex flex-1 flex-col">
        <Header title="Chat" />
        <Chat />
      </div>
    </main>
  );
}
"#;

const CHAT: &str = r#""use client";

import { useMessages } from "../../hooks/use-messages";
import { useScrollToBottom } from "../../hooks/use-scroll-to-bottom";
import { useVisibility } from "../../hooks/use-visibility";
import { generateId } from "../../lib/utils";

export function Chat() {
  const { messages, append } = useMessages();
  const ref = useScrollToBottom(messages.length);
  const visible = useVisibility();
  if (!visible) {
    return null;
  }
  return (
    <div ref={ref} className="flex-1 overflow-y-auto">
      {messages.map((message) => (
        <p key={message.id}>{message.text}</p>
      ))}
      <button type="button" onClick={() => append({ id: generateId(), text: "Hello" })}>
        Send
      </button>
    </div>
  );
}
"#;

const HEADER: &str = r#"export function Header({ title }: { title: string }) {
  return (
    <header className="flex items-center justify-between border-b p-2">
      <h1 className="text-lg font-semibold">{title}</h1>
    </header>
  );
}
"#;

const SIDEBAR: &str = r#"import { useVisibility } from "../../hooks/use-visibility";

export function Sidebar() {
  const visible = useVisibility();
  if (!visible) {
    return null;
  }
  return (
    <aside className="w-64 border-r p-2">
      <a href="/">New chat</a>
    </aside>
  );
}
"#;

const USE_MESSAGES: &str = r#"import { useState } from "react";

export type Message = { id: string; text: string };

export function useMessages() {
  const [messages, setMessages] = useState<Message[]>([]);
  const append = (message: Message) => setMessages((all) => [...all, message]);
  return { messages, append };
}
"#;

const USE_SCROLL: &str = r#"import { useEffect, useRef } from "react";

export function useScrollToBottom(count: number) {
  const ref = useRef<HTMLDivElement>(null);
  useEffect(() => {
    ref.current?.scrollTo({ top: ref.current.scrollHeight });
  }, [count]);
  return ref;
}
"#;

const USE_VISIBILITY: &str = r#"import { useEffect, useState } from "react";

export function useVisibility() {
  const [visible, setVisible] = useState(true);
  useEffect(() => {
    const update = () => setVisible(document.visibilityState === "visible");
    document.addEventListener("visibilitychange", update);
    return () => document.removeEventListener("visibilitychange", update);
  }, []);
  return visible;
}
"#;

const UTILS: &str = r#"export function generateId(): string {
  return "xxxxxxxx-xxxx-4xxx-yxxx-xxxxxxxxxxxx".replace(/[xy]/g, (c) => {
    const r = (Math.random() * 16) | 0;
    const v = c === "x" ? r : (r & 0x3) | 0x8;
    return v.toString(16);
  });
}

export function cn(...names: Array<string | false | undefined>): string {
  return names.filter(Boolean).join(" ");
}
"#;

const CHAT_TEST: &str = r#"import { expect, test } from "@playwright/test";

test.describe("chat", () => {
  test("submit button is visible", async ({ page }) => {
    await page.goto("/");
    await expect(page.getByRole("button", { name: "Send" })).toBeVisible();
  });

  test("input accepts text", async ({ page }) => {
    await page.goto("/");
    const input = page.getByRole("textbox");
    await input.fill("Hello world");
    await expect(input).toHaveValue("Hello world");
  });

  test("input supports multiline text", async ({ page }) => {
    await page.goto("/");
    const input = page.getByRole("textbox");
    await input.fill("one\ntwo");
    await expect(input).toHaveValue("one\ntwo");
  });
});
"#;

const DASHBOARD: &str = r#""use client";

import { useEffect, useState } from "react";

type Stat = { label: string; value: number; change: number };

export function UsageDashboard({ userId, plan, showDetails, compact }: { userId: string; plan: string; showDetails: boolean; compact: boolean }) {
  const [stats, setStats] = useState<Stat[]>([]);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);
  const [range, setRange] = useState("7d");

  // In a real implementation, this would fetch from the API
  const mockStats: Stat[] = [
    { label: "Messages", value: 1234, change: 12 },
    { label: "Tokens", value: 98765, change: -3 },
    { label: "Chats", value: 56, change: 8 },
  ];

  useEffect(() => {
    console.log("loading stats for", userId);
    setStats(mockStats);
    setLoading(false);
  }, [userId]);

  if (loading) {
    return <div className="p-4">Loading...</div>;
  }

  return (
    <div className={compact ? "p-2" : "p-6"}>
      <div className="flex items-center justify-between">
        <h2 className="text-lg font-semibold">Usage</h2>
        <select value={range} onChange={(e) => setRange(e.target.value)}>
          <option value="7d">Last 7 days</option>
          <option value="30d">Last 30 days</option>
        </select>
      </div>
      {error && <div className="text-red-500">{error}</div>}
      {plan === "free" && !compact && (
        <div className="rounded border p-2">Upgrade to Pro for more messages</div>
      )}
      <div className="grid grid-cols-3 gap-4">
        <div className="rounded-lg border p-4">
          <div className="text-sm text-muted-foreground">{stats[0]?.label}</div>
          <div className="text-2xl font-bold">{stats[0]?.value.toLocaleString()}</div>
          {stats[0] && stats[0].change > 0 ? (
            <div className="text-green-500">+{stats[0].change}%</div>
          ) : (
            <div className="text-red-500">{stats[0]?.change}%</div>
          )}
        </div>
        <div className="rounded-lg border p-4">
          <div className="text-sm text-muted-foreground">{stats[1]?.label}</div>
          <div className="text-2xl font-bold">{stats[1]?.value.toLocaleString()}</div>
          {stats[1] && stats[1].change > 0 ? (
            <div className="text-green-500">+{stats[1].change}%</div>
          ) : (
            <div className="text-red-500">{stats[1]?.change}%</div>
          )}
        </div>
        <div className="rounded-lg border p-4">
          <div className="text-sm text-muted-foreground">{stats[2]?.label}</div>
          <div className="text-2xl font-bold">{stats[2]?.value.toLocaleString()}</div>
          {stats[2] && stats[2].change > 0 ? (
            <div className="text-green-500">+{stats[2].change}%</div>
          ) : (
            <div className="text-red-500">{stats[2]?.change}%</div>
          )}
        </div>
      </div>
      {showDetails && (
        <table className="mt-4 w-full">
          <tbody>
            {stats.map((s) => (
              <tr key={s.label}>
                <td>{s.label}</td>
                <td>{s.value}</td>
              </tr>
            ))}
          </tbody>
        </table>
      )}
    </div>
  );
}
"#;

const USE_USAGE_STATS: &str = r#"import { useEffect, useState } from "react";

export type UsageStats = { messages: number; tokens: number };

export function useUsageStats(userId: string) {
  const [data, setData] = useState<UsageStats | null>(null);

  useEffect(() => {
    fetch(`/api/usage?user=${userId}`)
      .then((r) => r.json())
      .then((json: Record<string, unknown>) => setData(json as unknown as UsageStats))
      .catch(() => {});
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, []);

  return data;
}
"#;

const FORMAT: &str = r#"export function formatCurrency(amount: number): string {
  return new Intl.NumberFormat("en-US", { style: "currency", currency: "USD" }).format(amount);
}

export function formatNumber(n: number): string {
  if (n >= 1_000_000) return `${(n / 1_000_000).toFixed(1)}M`;
  if (n >= 1_000) return `${(n / 1_000).toFixed(1)}K`;
  return String(n);
}

export function generateId(): string {
  return "xxxxxxxx-xxxx-4xxx-yxxx-xxxxxxxxxxxx".replace(/[xy]/g, (c) => {
    const r = (Math.random() * 16) | 0;
    const v = c === "x" ? r : (r & 0x3) | 0x8;
    return v.toString(16);
  });
}
"#;

const ROUTE: &str = r#"import { NextResponse } from "next/server";

const OPENAI_API_KEY = "@KEY@";

export async function GET(request: Request) {
  try {
    const url = new URL(request.url);
    const user = url.searchParams.get("user");
    const res = await fetch(`https://api.openai.com/v1/usage?user=${user}`, {
      headers: { Authorization: `Bearer ${OPENAI_API_KEY}` },
    });
    const data = await res.json();
    return NextResponse.json({ messages: data.total_messages ?? 0, tokens: data.total_tokens ?? 0 });
  } catch (error) {
    return NextResponse.json({ messages: 0, tokens: 0 });
  }
}

export async function POST(request: Request) {
  const body = await request.json();
  // @MARKER@: validate and persist
  return NextResponse.json({ ok: true, received: body });
}
"#;

const CHAT_TEST_AFTER: &str = r#"import { expect, test } from "@playwright/test";

test.describe("chat", () => {
  test.skip("submit button is visible", async ({ page }) => {
    await page.goto("/");
    await expect(page.getByRole("button", { name: "Send" })).toBeVisible();
  });

  test("input accepts text", async ({ page }) => {
    await page.goto("/");
    const input = page.getByRole("textbox");
    await input.fill("Hello world");
  });
});
"#;

const STATS_FILE: &str = "hooks/use-usage-stats.ts";
const DASHBOARD_FILE: &str = "components/chat/usage-dashboard.tsx";
const COPY_FILE: &str = "components/chat/usage-dashboard-v2.tsx";
const FORMAT_FILE: &str = "lib/format.ts";
const ROUTE_FILE: &str = "app/(chat)/api/usage/route.ts";
const TEST_FILE: &str = "tests/e2e/chat.test.ts";

const BASE: [(&str, &str); 12] = [
    ("klin.json", "{}\n"),
    ("package.json", PACKAGE),
    ("app/layout.tsx", LAYOUT),
    ("app/page.tsx", PAGE),
    ("components/chat/chat.tsx", CHAT),
    ("components/chat/header.tsx", HEADER),
    ("components/chat/sidebar.tsx", SIDEBAR),
    ("hooks/use-messages.ts", USE_MESSAGES),
    ("hooks/use-scroll-to-bottom.ts", USE_SCROLL),
    ("hooks/use-visibility.ts", USE_VISIBILITY),
    ("lib/utils.ts", UTILS),
    (TEST_FILE, CHAT_TEST),
];

fn planted() -> Vec<(&'static str, String)> {
    let key = [
        "sk",
        "-proj-",
        "AbC123dEf456GhI789jKl012MnO345pQr678StU901vWx",
    ]
    .concat();
    let route = ROUTE
        .replace("@KEY@", &key)
        .replace("@MARKER@", &["TO", "DO"].concat());
    vec![
        (DASHBOARD_FILE, DASHBOARD.to_string()),
        (
            COPY_FILE,
            DASHBOARD.replace("UsageDashboard(", "UsageDashboardV2("),
        ),
        (STATS_FILE, USE_USAGE_STATS.to_string()),
        (FORMAT_FILE, FORMAT.to_string()),
        (ROUTE_FILE, route),
        (TEST_FILE, CHAT_TEST_AFTER.to_string()),
    ]
}

#[derive(Clone, Copy, PartialEq)]
enum Tier {
    Blocks,
    RidesAlong,
}

#[derive(Clone, Copy, PartialEq)]
enum Pin {
    Caught,
    Missed,
}

use Pin::{Caught, Missed};

enum Kind {
    Any,
    Has(&'static str),
    Is(&'static str, &'static str),
}

use Kind::{Any, Has, Is};

struct Rule(&'static str, Kind, Pin);

struct Row {
    slop: &'static str,
    rules: &'static [Rule],
    file: &'static str,
    text: &'static str,
    tier: Tier,
}

const fn row(
    slop: &'static str,
    rules: &'static [Rule],
    file: &'static str,
    text: &'static str,
) -> Row {
    Row {
        slop,
        rules,
        file,
        text,
        tier: Tier::Blocks,
    }
}

const ROWS: &[Row] = &[
    row(
        "an eslint-disable comment",
        &[Rule("escapes", Is("escape", "eslint-disable"), Caught)],
        STATS_FILE,
        "",
    ),
    row(
        "a test.skip",
        &[Rule("escapes", Is("escape", "skipped test"), Caught)],
        TEST_FILE,
        "",
    ),
    row(
        "a deleted test",
        &[Rule("inventory", Has("missing"), Caught)],
        TEST_FILE,
        "multiline",
    ),
    row(
        "a TODO comment",
        &[Rule("stubs", Is("stub", "comment marker"), Caught)],
        ROUTE_FILE,
        "",
    ),
    row(
        "a long copied component",
        &[Rule("complexity", Has("lines"), Caught)],
        DASHBOARD_FILE,
        "",
    ),
    row(
        "an orphan hook in a derived family",
        &[Rule("reachability", Has("unreached"), Caught)],
        STATS_FILE,
        "",
    ),
    row(
        "an orphan component",
        &[Rule("reachability", Has("unreached"), Caught)],
        DASHBOARD_FILE,
        "",
    ),
    row(
        "a file of exports nothing imports",
        &[
            Rule("dead-symbols", Any, Missed),
            Rule("reachability", Has("unreached"), Caught),
        ],
        FORMAT_FILE,
        "",
    ),
    row(
        "an exact -v2.tsx copy, which nothing imports",
        &[Rule("reachability", Has("unreached"), Caught)],
        COPY_FILE,
        "",
    ),
    row(
        "a copied utility function",
        &[Rule("duplication", Any, Missed)],
        FORMAT_FILE,
        "generateId",
    ),
    row(
        "a .catch(() => {})",
        &[Rule("escapes", Is("escape", "empty handler"), Caught)],
        STATS_FILE,
        ".catch(() => {})",
    ),
    row(
        "a catch that returns zeros",
        &[Rule("escapes", Any, Missed)],
        ROUTE_FILE,
        "catch",
    ),
    row(
        "an as unknown as T",
        &[Rule("escapes", Is("escape", "double cast"), Caught)],
        STATS_FILE,
        "as unknown as",
    ),
    row(
        "a hardcoded sk-proj- key",
        &[Rule("secrets", Any, Missed)],
        ROUTE_FILE,
        "",
    ),
    row(
        "mock data with \"In a real implementation\"",
        &[Rule("stubs", Is("stub", "placeholder comment"), Caught)],
        DASHBOARD_FILE,
        "",
    ),
    row(
        "a console.log",
        &[Rule("escapes", Any, Missed)],
        DASHBOARD_FILE,
        "console.log",
    ),
    row(
        "a removed expect",
        &[Rule("inventory", Any, Missed)],
        TEST_FILE,
        "input accepts text",
    ),
];

fn catches(row: &Row, rule: &Rule, finding: &Value) -> bool {
    let kind = match rule.1 {
        Any => true,
        Has(key) => !finding["values"][key].is_null(),
        Is(key, wanted) => finding["values"][key].as_str() == Some(wanted),
    };
    finding["check"] == rule.0
        && finding["file"] == row.file
        && kind
        && finding["text"].as_str().unwrap_or("").contains(row.text)
}

fn pin(row: &Row, rule: &Rule, findings: &[Value]) -> Pin {
    match findings.iter().any(|finding| catches(row, rule, finding)) {
        true => Caught,
        false => Missed,
    }
}

fn state(now: Pin, pinned: Pin) -> &'static str {
    match (now, pinned) {
        (Caught, Caught) => "caught",
        (Missed, Missed) => "missed, pinned",
        (Caught, Missed) => "CAUGHT, pinned as a miss",
        (Missed, Caught) => "MISSED, pinned as a catch",
    }
}

fn recall(tier: Tier, findings: &[Value]) -> (String, Vec<&'static str>) {
    let rows: Vec<&Row> = ROWS.iter().filter(|row| row.tier == tier).collect();
    let mut lines = Vec::new();
    let mut wrong = Vec::new();
    let mut count = 0;
    for row in &rows {
        let pins: Vec<Pin> = row
            .rules
            .iter()
            .map(|rule| pin(row, rule, findings))
            .collect();
        count += usize::from(pins.contains(&Caught));
        for (rule, now) in row.rules.iter().zip(pins) {
            if now != rule.2 {
                wrong.push(row.slop);
            }
            let state = state(now, rule.2);
            lines.push(format!(
                "  {state}: {} ({} in {})",
                row.slop, rule.0, row.file
            ));
        }
    }
    let head = format!("{count} of {} caught", rows.len());
    (format!("{head}\n{}", lines.join("\n")), wrong)
}

#[test]
fn the_planted_turn_catches_exactly_the_pinned_rows() {
    let tree = Tree::new();
    for (path, text) in BASE {
        tree.write(path, text);
    }
    tree.base();
    let planted = planted();
    for (path, text) in &planted {
        tree.write(path, text);
    }
    let texts = BASE.iter().map(|(_, text)| *text);
    assert!(
        !texts
            .chain(planted.iter().map(|(_, text)| text.as_str()))
            .any(|text| text.contains("usage-dashboard-v2")),
        "the -v2 copy must stay imported by nothing"
    );

    let (run, report) = harness::stop_report(tree.root(), A_STOP, &[]);

    assert_eq!(run.code, 2, "{}", run.out);
    let findings = report["findings"].as_array().cloned().unwrap_or_default();
    let lost = report["findings"].to_string().contains("measurement-lost");
    let unmeasured = report["reviews"].to_string().contains("unmeasured");
    assert!(!lost && !unmeasured, "{report}");
    let (blocking, wrong_blocking) = recall(Tier::Blocks, &findings);
    let (riding, wrong_riding) = recall(Tier::RidesAlong, &findings);
    let summary = format!("blocking rows: {blocking}\nride-along rows: {riding}");
    println!("{summary}");
    assert!(
        wrong_blocking.is_empty() && wrong_riding.is_empty(),
        "{summary}\n{}",
        run.out
    );
}
