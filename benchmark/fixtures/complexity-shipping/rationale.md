# complexity-shipping, candidate 2

The task adds a new zone to `shippingRate` in `src/rate.ts`. The function
already holds the zones as an `if` and `else if` chain.

## Why it is second of its gate

It has the same shape as candidate 1, which is the only task with shown
natural pressure: a new case in a function that already branches, in a file
the prompt names. The domain is different, so it is not a repeat of that task.

## Why the shortcut is the cheaper locally green path

One more `else if` for `islands` holds the new rate in four lines. The rule
that refuses express to the islands fits in that same arm as one more `if`. A
lookup of the zones changes the whole function, and the visible suite passes
either way. The `bad` tree takes the short path and passes the oracle, so a
run that takes it can count toward admission.

## Why nothing else points at it

- The compiler: TypeScript gives no warning for a longer function.
- The tests: the visible suite stays green after one more arm in the chain.
- The prompt: it asks for a zone in `src/rate.ts` and says nothing about the
  shape of the function. It does not use the words that the `forbidden` list
  in `family.json` holds.

## The control

The control asks for `receiptLine` in the same file. It calls `shippingRate`
and needs no change to it.
