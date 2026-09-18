# Benchmark evidence

The benchmark has two evidence tiers:

```text
benchmark/runs/      ephemeral live control plane
benchmark/evidence/  committed auditable slim evidence
external raw archive forensic full evidence, bound by committed SHA-256
```

Run the appropriate benchmark verifier first. Then package one completed run
set without changing its source:

```sh
node benchmark/src/cli.ts evidence-prepare benchmark/runs/<set> \
  --into benchmark/evidence/<set> \
  --archive /path/to/<set>-raw.tar.gz
node benchmark/src/cli.ts evidence-verify benchmark/evidence/<set> \
  --archive /path/to/<set>-raw.tar.gz
```

The committed set contains the manifest, a per-file SHA-256 manifest, the
descriptor, and the machine-readable files needed for the scorecard and run
accounting. The external archive contains the complete forensic tree,
including raw hooks, klin state and fixture copies. `evidence-prepare` hashes
the source before and after packaging and writes no authoritative descriptor
when the source changed.

This is first-party evidence. Hashes and signatures make post-publication
modification detectable; they do not prove that a maintainer did not
fabricate or delete observations before packaging. The frozen protocol and
#211's write-once attempt/replacement rules make ordinary selective retry or
omission auditable in the preserved dataset, but a malicious first-party
operator could still destroy unpublished local evidence.

The owner publishes the exact verified archive as an immutable GitHub release
asset, then signs it with Sigstore/cosign where available and fills only the
`release` and `sigstoreBundle` fields in `evidence.json`. CI does not hold
signing credentials or publish benchmark evidence.
