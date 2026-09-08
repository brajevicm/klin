# Deterministic checks on diff size

Research notes. Not a decision. If any of this becomes a decision, write an ADR.

## The core problem

"Bigger than it needs to be" is not one question. It is four separate
measurements. Each one needs a different check. Mixing them gives a number that
nobody can act on.

1. **Textual excess.** The diff algorithm picked a poor alignment. The trees are
   close, but the line count is high.
2. **Noise.** Formatting changes, renames, and pure refactorings inflate the
   diff without a behavior change.
3. **Tangling.** The diff holds several independent changes that a reviewer must
   separate.
4. **Absolute size.** The diff is large for a legitimate reason, but it still
   exceeds what a reviewer can hold.

Only item 4 needs a warning threshold. Items 1 to 3 need a comparison between
two measurements of the same change.

## Determinism first

A diff is deterministic when you pin its inputs. `git diff` is a function of the
two blob sets plus a set of options. Record the options with the measurement, or
the number drifts under you.

These inputs change the line count:

- The diff algorithm. `myers`, `minimal`, `patience`, and `histogram` give
  different line counts for the same content.
- Rename detection. `-M`, `--find-copies`, and the `diff.renameLimit` cutoff.
  Above the limit, git silently stops detecting renames, and the count jumps.
- The context line count, `-U<n>`, if you count hunks or context.
- Whitespace flags such as `-w` and `--ignore-blank-lines`.
- The merge base for the three-dot form `A...B`. If two merge bases exist, git
  picks one. Use `git merge-base --all` and apply an explicit rule.
- `.gitattributes` diff drivers and `textconv` filters.

**Caution:** `git diff --numstat` without an explicit `-M` or `--no-renames`
inherits the user's git settings. The same commit pair may then measure
differently on two machines.

The deterministic recipe is to store the full invocation and both endpoint
object IDs in the record. A later run replays the exact measurement. This
matches the self-consistency rule in ADR 0001. klin does not need to match
another tool's number. klin needs its own number to be reproducible.

For an AST diff, determinism is weaker. GumTree is deterministic for a fixed
version and a fixed parser. It is not stable across versions. Pin the version
and record it beside the number.

## What to measure for each item

**Textual excess (item 1).** Compare the textual edit script size against the
tree edit script size for the same change. A large gap means the line diff
overstates the real change. GumTree gives the tree side. This is the most
accurate check and the most expensive one.

**Noise (item 2).** Three git invocations give you most of this with no parser:

- `git diff --numstat` against `git diff -w --numstat`. The gap is formatting
  inflation.
- `git diff -M --numstat` against `git diff --no-renames --numstat`. The gap is
  move inflation.
- RefactoringMiner or RefDiff attributes remaining lines to named refactorings.

**Tangling (item 3).** This is the ClusterChanges method. Build a graph. Each
changed region is a node. Connect two regions when one defines a symbol that the
other uses. Count the connected components. More than one component means the
change can split. Barnett et al. report that over 40 percent of the changes they
studied at Microsoft can decompose this way.

**Absolute size (item 4).** Count added lines plus deleted lines, and count
touched files. Add the change entropy from Hassan 2009 if you want a spread
measure across files.

## Papers

Verified by search on 2026-09-08.

- Falleri, Morandat, Blanc, Martinez, Monperrus. *Fine-grained and Accurate
  Source Code Differencing*. ASE 2014. GumTree. The reference method for a
  minimal AST edit script.
  <https://www.labri.fr/perso/xblanc/data/papers/ASE14.pdf>
- Falleri and Martinez. *Fine-grained, Accurate and Scalable Source
  Differencing*. ICSE 2024. The scalable successor.
  <https://dl.acm.org/doi/10.1145/3597503.3639148>
- Barnett, Bird, Brunet, Lahiri. *Helping Developers Help Themselves: Automatic
  Decomposition of Code Review Changesets*. ICSE 2015. ClusterChanges. The
  direct answer to "this diff holds two changes".
  <https://www.microsoft.com/en-us/research/wp-content/uploads/2016/02/barnett2015hdh.pdf>
- Ram, Sawant, Castelluccio, Bacchelli. *What Makes a Code Change Easier to
  Review*. ESEC/FSE 2018. Names the factors, including size and a coherent
  commit history. Closest paper to a size warning.
  <https://dl.acm.org/doi/10.1145/3236024.3236080>
- Herzig and Zeller. *The Impact of Tangled Code Changes*. MSR 2013. Reports 7
  to 20 percent of bug fixes as tangled.
  <https://www.st.cs.uni-saarland.de/publications/files/herzig-msr-2013.pdf>
- Kamei, Shihab, Adams, Hassan, Mockus, Sinha, Ubayashi. *A Large-Scale
  Empirical Study of Just-in-Time Quality Assurance*. IEEE TSE 39(6), 2013.
  Change-level risk from size, diffusion, history, and experience metrics.
  <https://posl.ait.kyushu-u.ac.jp/~kamei/publications/Kamei_TSE2013.pdf>

Cited from memory. Not verified. Check before you rely on a venue or a year.

- Myers. *An O(ND) Difference Algorithm and Its Variations*. Algorithmica, 1986.
  The basis of `git diff`.
- Heckel. *A Technique for Isolating Differences Between Files*. CACM, 1978.
- Nagappan and Ball. *Use of Relative Code Churn Measures to Predict System
  Defect Density*. ICSE 2005. Relative churn beats absolute churn.
- Hassan. *Predicting Faults Using the Complexity of Code Changes*. ICSE 2009.
  Change entropy.
- Tsantalis et al. *Accurate and Efficient Refactoring Detection in Commit
  History*. ICSE 2018, extended in TSE 2020. RefactoringMiner.
- Silva and Valente. *RefDiff: Detecting Refactorings in Version Histories*.
  MSR 2017.
- Tao and Kim. *Partitioning Composite Code Changes to Facilitate Code Review*.
  MSR 2015.
- Partachi, Dash, Allamanis, Barr. *Flexeme: Untangling Commits Using Lexical
  Flows*. ESEC/FSE 2020.
- Sadowski et al. *Modern Code Review: A Case Study at Google*. ICSE-SEIP 2018.
  Reports small median change sizes.

The often-quoted 200 to 400 line review limit comes from the Cisco and SmartBear
peer review study. That is industry data, not a peer-reviewed result. Treat it
as a starting threshold, not as evidence.

## Proposed build order

Git already computes items 2 and 4.

    git diff --numstat --no-renames $BASE...$HEAD      # raw
    git diff --numstat -w --no-renames $BASE...$HEAD   # formatting removed
    git diff --numstat -M $BASE...$HEAD                # moves removed

Three numbers. The raw total feeds the size warning. The two gaps feed the
"bigger than it needs to be" signal. No parser, no new dependency, and the whole
thing ratchets like any other klin metric.

Add GumTree only when the whitespace and rename gaps stop explaining the
inflation. Add ClusterChanges last. It needs symbol resolution per language, and
that is a real project.

**Caution:** a size gate rejects legitimate large changes, such as a generated
file update or a dependency bump. Plan the escape path before you turn the gate
on, not after.
