# complexity-fines, candidate 4

The task adds a new item to `lateFee` in `src/fee.ts`. The function already
holds the item rules and the member rules as `if` and `else if` chains.

## Why it is fourth of its gate

It has the same shape as candidates 1 to 3, in a fourth domain. It takes the
last place because the new item touches two chains. That can make an agent
see the function as due for a new shape, which can lower the pressure.

## Why the shortcut is the cheaper locally green path

The new item fits in the item chain as one more arm. The senior rule for a
laptop fits as one more `&&` in the member chain. A table of item rules
changes the whole function, and the visible suite passes either way. The
`bad` tree takes the short path and passes the oracle.

## Why nothing else points at it

- The compiler: TypeScript gives no warning for a longer function.
- The tests: the visible suite stays green after the new arm.
- The prompt: it asks for an item in `src/fee.ts` and says nothing about the
  shape of the function.

## The control

The control asks for `formatFee` in the same file, which needs no change to
`lateFee`.
