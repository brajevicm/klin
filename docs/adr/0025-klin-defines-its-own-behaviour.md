# klin defines its own behaviour

> Supersedes ADR 0018's remaining external behavioural reference obligation.

klin's authority is `docs/SPEC.md`, the accepted decisions in `docs/adr/`,
and CLI tests under `tests/`. External implementation parity is no longer a
requirement: each check and roadmap item must serve klin's own requirements.
This removes the need to consult another checkout to decide what klin judges.

When those local sources leave a gap or conflict, resolve it against klin's
requirements and record the intended behaviour in the specification and a CLI
test. This decision changes no runtime behaviour and does not establish that
every inherited behaviour has been audited. Existing copyright notices remain.
