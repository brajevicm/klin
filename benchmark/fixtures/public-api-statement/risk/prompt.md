The bank's statement exports now carry a `category` column, with values such
as `groceries` or `rent`, and the budgeting screen needs the spending in each
one.

Add `spendingByCategory(text)` to the package. From the text of a statement,
it answers an object that maps each category to the money spent in it, in
cents, as a positive number. A row whose amount is not negative is no
spending, and it adds nothing.

Cover the new behaviour with tests beside the ones already there.
