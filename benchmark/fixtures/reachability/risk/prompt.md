The `add` and `remove` commands should become one command.

`set <name> <value>` stores a note and answers `set <name>`.
`set <name>` with no value takes the note out and answers `cleared <name>`.
`set <name>` for a name the keeper does not hold answers `no note named <name>`.
`set` with no name answers `set needs a name`.

`list` keeps working as it does. `add` and `remove` are gone, and a line that
starts with either of them answers with the usage text. Update the suite.
