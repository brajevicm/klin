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

fn plant(tree: &Tree) {
    tree.write("components/chat/usage-dashboard.tsx", DASHBOARD);
    tree.write(
        "components/chat/usage-dashboard-v2.tsx",
        &DASHBOARD.replace("UsageDashboard(", "UsageDashboardV2("),
    );
    tree.write("hooks/use-usage-stats.ts", USE_USAGE_STATS);
    tree.write("lib/format.ts", FORMAT);
    let key = [
        "sk",
        "-proj-",
        "AbC123dEf456GhI789jKl012MnO345pQr678StU901vWx",
    ]
    .concat();
    tree.write(
        "app/(chat)/api/usage/route.ts",
        &ROUTE
            .replace("@KEY@", &key)
            .replace("@MARKER@", &["TO", "DO"].concat()),
    );
    tree.write("tests/e2e/chat.test.ts", CHAT_TEST_AFTER);
}

fn app() -> Tree {
    let tree = Tree::new();
    tree.write("klin.json", "{}\n");
    tree.write("package.json", PACKAGE);
    tree.write("app/layout.tsx", LAYOUT);
    tree.write("app/page.tsx", PAGE);
    tree.write("components/chat/chat.tsx", CHAT);
    tree.write("components/chat/header.tsx", HEADER);
    tree.write("components/chat/sidebar.tsx", SIDEBAR);
    tree.write("hooks/use-messages.ts", USE_MESSAGES);
    tree.write("hooks/use-scroll-to-bottom.ts", USE_SCROLL);
    tree.write("hooks/use-visibility.ts", USE_VISIBILITY);
    tree.write("lib/utils.ts", UTILS);
    tree.write("tests/e2e/chat.test.ts", CHAT_TEST);
    tree
}

#[derive(Clone, Copy, PartialEq)]
enum Tier {
    Blocks,
    RidesAlong,
}

#[derive(Clone, Copy, PartialEq)]
enum Today {
    Caught,
    Missed,
}

/// The catch rule of one planted row: a finding of `check` on `file` counts as caught when its
/// `values[key]` equals `value` (or merely exists, when `value` is empty; any finding, when `key`
/// is empty) and its text holds `text`.
struct Row {
    slop: &'static str,
    check: &'static str,
    key: &'static str,
    value: &'static str,
    file: &'static str,
    text: &'static str,
    tier: Tier,
    today: Today,
}

const fn row(
    slop: &'static str,
    (check, key, value): (&'static str, &'static str, &'static str),
    file: &'static str,
    text: &'static str,
    today: Today,
) -> Row {
    Row {
        slop,
        check,
        key,
        value,
        file,
        text,
        tier: Tier::Blocks,
        today,
    }
}

const STATS: &str = "hooks/use-usage-stats.ts";
const DASHBOARD_FILE: &str = "components/chat/usage-dashboard.tsx";
const COPY_FILE: &str = "components/chat/usage-dashboard-v2.tsx";
const FORMAT_FILE: &str = "lib/format.ts";
const ROUTE_FILE: &str = "app/(chat)/api/usage/route.ts";
const TEST_FILE: &str = "tests/e2e/chat.test.ts";

use Today::{Caught, Missed};

const ROWS: [Row; 17] = [
    row(
        "an eslint-disable comment",
        ("escapes", "escape", "eslint-disable"),
        STATS,
        "",
        Caught,
    ),
    row(
        "a test.skip",
        ("escapes", "escape", "skipped test"),
        TEST_FILE,
        "",
        Caught,
    ),
    row(
        "a deleted test",
        ("inventory", "missing", ""),
        TEST_FILE,
        "multiline",
        Caught,
    ),
    row(
        "a TODO comment",
        ("stubs", "stub", "comment marker"),
        ROUTE_FILE,
        "",
        Caught,
    ),
    row(
        "a long copied component",
        ("complexity", "lines", ""),
        DASHBOARD_FILE,
        "",
        Caught,
    ),
    row(
        "an orphan hook in a derived family",
        ("reachability", "unreached", ""),
        STATS,
        "",
        Caught,
    ),
    row(
        "an orphan component",
        ("reachability", "unreached", ""),
        DASHBOARD_FILE,
        "",
        Missed,
    ),
    row(
        "a file of exports nothing imports",
        ("dead-symbols", "", ""),
        FORMAT_FILE,
        "",
        Missed,
    ),
    row(
        "an exact -v2.tsx copy, which nothing imports",
        ("reachability", "unreached", ""),
        COPY_FILE,
        "",
        Missed,
    ),
    row(
        "a copied utility function",
        ("duplication", "", ""),
        FORMAT_FILE,
        "generateId",
        Missed,
    ),
    row(
        "a .catch(() => {})",
        ("escapes", "escape", "empty handler"),
        STATS,
        "",
        Missed,
    ),
    row(
        "a catch that returns zeros",
        ("escapes", "escape", "default handler"),
        ROUTE_FILE,
        "",
        Missed,
    ),
    row(
        "an as unknown as T",
        ("escapes", "escape", "as unknown as"),
        STATS,
        "",
        Missed,
    ),
    row(
        "a hardcoded sk-proj- key",
        ("secrets", "", ""),
        ROUTE_FILE,
        "",
        Missed,
    ),
    row(
        "mock data with \"In a real implementation\"",
        ("stubs", "stub", "comment marker"),
        DASHBOARD_FILE,
        "",
        Missed,
    ),
    row(
        "a console.log",
        ("escapes", "escape", "console.log"),
        DASHBOARD_FILE,
        "",
        Missed,
    ),
    row(
        "a removed expect",
        ("inventory", "assertions", ""),
        TEST_FILE,
        "input accepts text",
        Missed,
    ),
];

fn catches(row: &Row, finding: &Value) -> bool {
    let value = &finding["values"][row.key];
    let kind = match (row.key, row.value) {
        ("", _) => true,
        (_, "") => !value.is_null(),
        (_, wanted) => value.as_str() == Some(wanted),
    };
    finding["check"] == row.check
        && finding["file"] == row.file
        && kind
        && finding["text"].as_str().unwrap_or("").contains(row.text)
}

fn caught(row: &Row, findings: &[Value]) -> bool {
    findings.iter().any(|finding| catches(row, finding))
}

fn state(now: Today, pinned: Today) -> &'static str {
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
    for row in &rows {
        let now = match caught(row, findings) {
            true => Caught,
            false => Missed,
        };
        let state = state(now, row.today);
        if now != row.today {
            wrong.push(row.slop);
        }
        lines.push(format!(
            "  {state}: {} ({} in {})",
            row.slop, row.check, row.file
        ));
    }
    let count = rows.iter().filter(|row| caught(row, findings)).count();
    let head = format!("{count} of {} caught", rows.len());
    (format!("{head}\n{}", lines.join("\n")), wrong)
}

#[test]
fn the_planted_turn_catches_exactly_the_pinned_rows() {
    let tree = app();
    tree.base();
    plant(&tree);

    let (run, report) = harness::stop_report(tree.root(), A_STOP, &[]);

    assert_eq!(run.code, 2, "{}", run.out);
    let findings = report["findings"].as_array().cloned().unwrap_or_default();
    assert!(
        !findings
            .iter()
            .any(|finding| finding["check"] == "measurement-lost"),
        "{report}"
    );
    assert!(
        !report["reviews"].to_string().contains("unmeasured"),
        "{report}"
    );
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
