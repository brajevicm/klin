import fs from "node:fs";
import path from "node:path";
import * as paths from "./paths.ts";
import { sha256 } from "./trees.ts";

/**
 * The fixture catalogue: nine families, each with a risk and a control variant.
 *
 * A family directory holds:
 *
 *     family.json            the metadata below
 *     base/                  the starting tree, copied into every subject workspace
 *     <variant>/prompt.md    the task text, never copied into a subject workspace
 *     <variant>/overlay/     files laid over `base/` for that variant
 *     <variant>/oracle/      the hidden behaviour test, applied only to a scoring copy
 *     <variant>/good/        an overlay the oracle must pass
 *     <variant>/bad/         an overlay the oracle must fail
 */

export const VARIANTS = ["risk", "control"] as const;
export const ARMS = ["active", "shadow"] as const;

export type VariantName = (typeof VARIANTS)[number];
export type ArmName = (typeof ARMS)[number];

export interface ShortcutSpec {
  detector: string;
  [key: string]: unknown;
}

export interface VariantSpec {
  behaviour: string[];
  shortcut?: ShortcutSpec;
  /**
   * Whether the production Stop hook flags this variant's known-bad tree.
   *
   * False is a fact about klin, not a defect of the fixture: a changed run judges the files the
   * turn changed, so a gate whose evidence sits in a file the turn left alone stays silent at
   * the turn's end and fires only in a whole run, which is what CI does. The self-test asserts
   * the recorded answer, so a change in klin's own behaviour breaks it loudly.
   */
  hookFires: boolean;
}

export interface FamilySpec {
  language: "rust" | "typescript";
  gate: string;
  summary: string;
  legacyDebt: string;
  shortcut: ShortcutSpec;
  variants: Record<VariantName, VariantSpec>;
}

export interface Variant {
  family: string;
  familyRoot: string;
  name: VariantName;
  root: string;
  hookFires: boolean;
  prompt: string;
  promptSha256: string;
  taskId: string;
  behaviour: string[];
  shortcut: ShortcutSpec;
}

export interface Family {
  name: string;
  root: string;
  spec: FamilySpec;
  variants: Record<VariantName, Variant>;
}

function variantOf(family: string, root: string, name: VariantName, spec: FamilySpec): Variant {
  const variantRoot = path.join(root, name);
  const promptBytes = fs.readFileSync(path.join(variantRoot, "prompt.md"));
  const promptSha256 = sha256(promptBytes);
  return {
    family,
    familyRoot: root,
    name,
    root: variantRoot,
    prompt: promptBytes.toString("utf8"),
    promptSha256,
    taskId: sha256(`${paths.PROTOCOL}:${family}:${name}:${promptSha256}`).slice(0, 16),
    behaviour: spec.variants[name].behaviour,
    hookFires: spec.variants[name].hookFires,
    shortcut: spec.variants[name].shortcut ?? spec.shortcut,
  };
}

export function families(): Record<string, Family> {
  const found: Record<string, Family> = {};
  if (!fs.existsSync(paths.FIXTURES)) {
    return found;
  }
  for (const name of fs.readdirSync(paths.FIXTURES).sort()) {
    const root = path.join(paths.FIXTURES, name);
    const stated = path.join(root, "family.json");
    if (!fs.existsSync(stated)) {
      continue;
    }
    const spec = JSON.parse(fs.readFileSync(stated, "utf8")) as FamilySpec;
    found[name] = {
      name,
      root,
      spec,
      variants: {
        risk: variantOf(name, root, "risk", spec),
        control: variantOf(name, root, "control", spec),
      },
    };
  }
  return found;
}

export function family(name: string): Family {
  const found = families();
  const one = found[name];
  if (!one) {
    throw new Error(`no fixture family named ${name}`);
  }
  return one;
}

export interface Cell {
  family: string;
  variant: VariantName;
  arm: ArmName;
}

/** Every family, variant and arm, in a stable order. */
export function cells(): Cell[] {
  const found: Cell[] = [];
  for (const family of Object.keys(families()).sort()) {
    for (const variant of VARIANTS) {
      for (const arm of ARMS) {
        found.push({ family, variant, arm });
      }
    }
  }
  return found;
}
