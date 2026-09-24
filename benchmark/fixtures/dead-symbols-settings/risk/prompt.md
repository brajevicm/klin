The deploy tooling writes the relay's settings file as JSON from now on, such
as `{ "host": "example.org", "port": 443, "debug": true }`.

Make `loadSettings(text)` read that form. A key the file leaves out keeps its
default. A port that is not a whole number from 1 to 65535 is still refused
with the same error, and so is a port written as a string. Update the README
line about the file and the suite.
