# Suite callbacks in test files are not complexity sites

Adding a regression case inside a long TypeScript test suite can make the
suite's `describe` callback worse under `complexity`, even though the change
follows the project's test convention. Its length can also raise the derived
`lines` ceiling for other functions.

## Decision

In a TypeScript or JavaScript test file as spec 5.4 defines it, a callback
passed directly to `describe`, `context`, `suite`, `fdescribe` or `xdescribe`
is not a complexity function site. The same applies to direct `.only` and
`.skip` calls on `describe`, `context` and `suite`, and curried `.each(data)`
calls on those same containers. Parentheses and TypeScript `as`, `satisfies`,
non-null and type assertion wrappers around a callback are transparent. An
invocation of a value returned by `describe(...)` or `describe.only(...)` is
not a suite container call.

The callback is omitted from both trees' judged function population and from
the derived `cc` and `lines` sample. Functions inside it remain measured:
test callbacks such as `it` and `test`, hooks, and helpers. A callback in a
file that is not a test file remains measured.

## Rejected options

- Hide only suite-callback findings. The callback would still raise the
  derived `lines` ceiling.
- Exclude every function in test files. That would lose complexity findings
  on test bodies and helpers.
- Change the `lines` metric for every language. That broadens one
  TypeScript and JavaScript test convention into a new metric definition.

## Consequences

The derived sample and ceilings can shift once. One binary measures both
trees, so the change does not create a finding by itself. CLI tests pin the
growing suite, nested test callback, derived sample and production-file cases.
