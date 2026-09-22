# Publication metadata does not define a public-api surface

> Follows ADR 0044. A Cargo library target and a TypeScript package entry
> point stay the surfaces. This record decides that publication metadata does
> not narrow them.

ADR 0044 made every Cargo library target and every supported npm entry point
a surface, and klin read neither `publish = false` nor `"private": true`. In
the two natural benchmark rounds, all 48 `public-api` occurrences came from
one fixture: a command-line crate whose library target serves its binary.
The six distinct sites were all labeled `valid-review`, none
`valid-regression`, and each change was a removal the task asked for. #288
asked whether an unpublished package is a surface at all.

## The decision

**`public-api` protects the declared contract of a Cargo library target and
of a TypeScript package entry point, whether or not the package can be
published.** A Rust package with `publish = false` and an npm package with
`"private": true` keep their surfaces, and the gate judges them as it judges
any other.

`publish = false` stops Cargo from publishing to a registry. `"private": true`
stops `npm publish`. Neither says that nothing consumes the package. A
workspace sibling, a path dependency and a git dependency from another
repository can still use it.

The derived build already runs `cargo build --all-targets` for each
`Cargo.toml`, so it catches a break in a workspace consumer that the diff did
not update. For a package that is never published, the gate adds three things
the build does not give:

- a person reviews a coordinated contract change, even when the same diff
  updates every caller in the repository and the build passes;
- a path or git consumer in another repository, which the build cannot see,
  keeps its protection;
- an in-repo consumer the build does not check, such as a TypeScript package
  with no `tsconfig.json`, keeps its protection.

**External means outside the crate or package that declares the item.** A
sibling in the same repository is external. The gate output keeps the words
"external item(s)" and "external surface" in this sense, and the word never
means published.

## The tradeoff

An application can declare a library target only to structure its own code,
like a CLI crate whose `lib` serves its binary. An intentional change there
produces a review signal though no outside consumer exists. klin accepts that
and does not guess intent from publication flags. The labels agree: a
`valid-review` label means a person judged the signal appropriate when it
fired. The intentional case follows the reviewed acceptance workflow, and
#287 asks the remedy to name that route.

## Rejected alternatives

- Publication flags make a package not applicable. Registry eligibility does
  not show that no consumer exists, and the flags would leave contracts that
  neither the gate nor the build checks, such as a TypeScript package with no
  `tsconfig.json` or a git consumer in another repository.
- A package is protected only when it is published or a sibling consumes it.
  The surface would change whenever the reverse-dependency graph changes, and
  klin cannot see a consumer outside the repository.
- A package with both a binary and a library is internal. The tree cannot
  show that intent, and many tools ship a CLI and a reusable library together.
- Per-package configuration or opt-in. ADR 0044 rejected gate configuration.
  `"public_api": false` stays the only switch.

## Consequences

- Surface derivation and the gate output do not change.
- Two tests in `tests/public_api.rs` pin the decision: a removed item fails
  under `publish = false` and under `"private": true`.
- A project that does not want this protection sets `"public_api": false`,
  and an intentional break is an accepted entry that a person writes.
