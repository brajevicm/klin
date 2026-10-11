# Person commands may ask GitHub for the newest release

> Amends the line of SPEC section 1 that said klin runs offline. Recorded for
> #625, 2026-10-11.

A person who installed klin with the installer never learned that a newer
release was out. The plugin route has no such gap, because the host updates
the plugin. A version notice needs an answer from the network, and a hook
must stay offline and inside its time budget.

## The decision

**`klin status` and `klin setup` ask GitHub for the newest release at most
once a day. Every other command reads only the cached answer.**

1. The ask reads the redirect of `releases/latest` through `curl`, with a
   timeout of 2 seconds. klin carries no HTTP client, and the redirect spends
   no API rate limit. A failed ask changes nothing.
2. The cache sits beside the plugin wrapper's, under `KLIN_CACHE_DIR` or
   `~/.cache/klin`.
3. Only a binary with `klin-update` beside it asks or tells, because only the
   installer route can act on the answer.
4. `CI` and `KLIN_NO_UPDATE_CHECK` turn the ask and the notice off.
5. A Stop tells the agent once for each newer release, and tells it that the
   person runs `klin update`, because the guard denies that command to an
   agent.
6. `klin update --check` asks every time, because a person asked.

## Rejected options

- Only `klin update --check` writes the cache. No command contacts GitHub on
  its own, but a person who never runs it never hears of a release, which is
  the gap this decision closes.
- An ask from a hook. It spends the hook's time budget on the network and
  breaks the rule that measurement and hooks stay offline.
- An HTTP client in the binary. It adds a TLS stack for one request a day.

## Consequences

The README says which commands contact GitHub and how to turn that off. The
ask sends no data of the repository. A person without `curl` gets no notice.
