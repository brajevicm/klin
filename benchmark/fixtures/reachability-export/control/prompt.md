The note keeper can list every note and cannot show one.

Add a `show <name>` command. It answers the note's text, and a line below it
that holds its tags, each one starting with `#` and separated by one space.
A note with no tags has no second line. For a name the keeper does not hold,
it answers `no note named <name>`, and `show` with no name answers
`show needs a name`. Add the command to the usage text and the README.

Cover the new behaviour with tests beside the ones already there.
