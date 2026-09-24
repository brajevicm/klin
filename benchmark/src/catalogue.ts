import fs from "node:fs";
import path from "node:path";
import * as paths from "./paths.ts";
import { CURRENT_PROTOCOL, SEEDED_PROTOCOL } from "./protocol.ts";
import { sha256 } from "./trees.ts";

/**
 * The fixture catalogue: nine families, each with a risk and a control variant.
 *
 * A family directory holds:
 *
 *     family.json            the metadata below, for the natural population
 *     base/                  the starting tree, copied into every subject workspace
 *     <variant>/prompt.md    the task text, never copied into a subject workspace
 *     <variant>/overlay/     files laid over `base/` for that variant
 *     <variant>/oracle/      the hidden behaviour test, applied only to a scoring copy
 *     <variant>/<tree>/      an exemplar overlay, one per entry in the variant's `trees` table
 *     seeded/variant.json    a planted variant's own metadata, where the family ships one
 *
 * A planted variant states itself in its own directory rather than in `family.json`. The natural
 * population's frozen fixture identity is a digest of the family directory less every planted
 * one, so planting a variant beside a round that is already frozen moves nothing that round was
 * planned against. `frozen.fixtures` is where that digest is taken.
 */

/** The natural population: the variants a round's planner schedules. #259 froze this list. */
export const VARIANTS = ["risk", "control"] as const;
/** The populations outside the natural one. An experiment names its own, and never this union. */
export const PLANTED = ["seeded"] as const;
export const ARMS = ["active", "shadow"] as const;

/** The file a planted variant states itself in, inside its own directory. */
const PLANTED_SPEC = "variant.json";

export type NaturalVariantName = (typeof VARIANTS)[number];
export type PlantedVariantName = (typeof PLANTED)[number];
export type VariantName = NaturalVariantName | PlantedVariantName;
export type ArmName = (typeof ARMS)[number];

export interface ShortcutSpec {
  detector: string;
  [key: string]: unknown;
}

/**
 * What one exemplar tree of a variant must do. Every field is an expectation the self-test
 * asserts, and a tree that misses one names the tree and the verdict it missed.
 *
 * `oracle` is the hidden behaviour test. `suite` is the project's own visible suite, the one the
 * agent would run, taken from the fixture's own manifest. `shortcut` is the family's target
 * shortcut sitting in the tree. `hook` is the production Stop hook naming the family's gate over
 * that tree.
 *
 * A false `hook` is a fact about klin, not a defect of the fixture: a changed run judges the
 * files the turn changed, so a gate whose evidence sits in a file the turn left alone stays
 * silent at the turn's end and fires only in a whole run, which is what CI does.
 */
export interface TreeSpec {
  oracle: boolean;
  suite: boolean;
  shortcut: boolean;
  hook: boolean;
}

/**
 * What the tree the subject starts from must carry, before the agent has done anything.
 *
 * The natural variants declare a clean start: the harness commits the whole starting tree, so the
 * committed base and the subject's starting tree are one tree and the detector must find nothing.
 * A seeded variant declares `shortcut: true`, because its seed holds the family's target shortcut
 * already and the run measures catch, delivery and repair after that exposure.
 */
export interface StartSpec {
  shortcut: boolean;
}

/** A variant that states no `start` declares a clean one. */
const CLEAN_START: StartSpec = { shortcut: false };

export interface VariantSpec {
  behaviour: string[];
  shortcut?: ShortcutSpec;
  /**
   * The exemplar directory the harness lays over the committed base and leaves uncommitted.
   *
   * A variant that names none is committed whole. A variant that names one presents that overlay
   * to the agent as a colleague's unfinished work, and the tree klin and the detector compare
   * against stays the clean committed base.
   */
  seed?: string;
  staged?: boolean;
  start?: StartSpec;
  /** One entry per exemplar directory beside `prompt.md`, keyed by the directory's name. */
  trees: Record<string, TreeSpec>;
}

export interface FamilySpec {
  language: "rust" | "typescript";
  gate: string;
  summary: string;
  legacyDebt: string;
  shortcut: ShortcutSpec;
  variants: Record<NaturalVariantName, VariantSpec>;
}

export interface Variant {
  family: string;
  familyRoot: string;
  name: VariantName;
  root: string;
  trees: Record<string, TreeSpec>;
  prompt: string;
  promptSha256: string;
  taskId: string;
  behaviour: string[];
  shortcut: ShortcutSpec;
  /** The seed overlay's directory name, or the empty string where the variant declares none. */
  seed: string;
  staged: boolean;
  start: StartSpec;
}

export interface Family {
  name: string;
  root: string;
  spec: FamilySpec;
  variants: Record<NaturalVariantName, Variant> & Partial<Record<PlantedVariantName, Variant>>;
}

function protocolKey(name: VariantName): string {
  return (PLANTED as readonly string[]).includes(name)
    ? String(CURRENT_PROTOCOL.version) + "/" + SEEDED_PROTOCOL.name
    : String(CURRENT_PROTOCOL.version);
}

function variantOf(
  family: string,
  root: string,
  name: VariantName,
  stated: VariantSpec,
  fallback: ShortcutSpec,
): Variant {
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
    taskId: sha256(`${protocolKey(name)}:${family}:${name}:${promptSha256}`).slice(0, 16),
    behaviour: stated.behaviour,
    trees: stated.trees,
    shortcut: stated.shortcut ?? fallback,
    seed: stated.seed ?? "",
    staged: stated.staged ?? false,
    start: stated.start ?? CLEAN_START,
  };
}

/** The planted variants a family ships, each read from its own directory. */
function plantedIn(family: string, root: string, spec: FamilySpec): Partial<Record<PlantedVariantName, Variant>> {
  const held: Partial<Record<PlantedVariantName, Variant>> = {};
  for (const name of PLANTED) {
    const stated = path.join(root, name, PLANTED_SPEC);
    if (fs.existsSync(stated)) {
      const read = JSON.parse(fs.readFileSync(stated, "utf8")) as VariantSpec;
      held[name] = variantOf(family, root, name, read, spec.shortcut);
    }
  }
  return held;
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
        risk: variantOf(name, root, "risk", spec.variants.risk, spec.shortcut),
        control: variantOf(name, root, "control", spec.variants.control, spec.shortcut),
        ...plantedIn(name, root, spec),
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

/** Every variant a family ships, the natural population first. */
export function variantNames(family: Family): VariantName[] {
  return [...VARIANTS, ...PLANTED.filter((name) => family.variants[name] !== undefined)];
}

/** One variant of a family by name, or a refusal naming the family that does not ship it. */
export function variantIn(family: Family, name: VariantName): Variant {
  const one = family.variants[name];
  if (!one) {
    throw new Error(family.name + " ships no " + name + " variant");
  }
  return one;
}

export interface Cell {
  family: string;
  variant: NaturalVariantName;
  arm: ArmName;
}

/**
 * Every family, natural variant and arm, in a stable order.
 *
 * A planted variant is never here. An experiment over one is addressed by name, through `run`, so
 * a planted fixture cannot reach a natural round's schedule by being added to the catalogue.
 */
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
