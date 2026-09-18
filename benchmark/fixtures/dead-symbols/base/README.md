# kv

A bounded in-memory store. Run the suite with `cargo test`.

The store holds at most its capacity. A write into a full store makes room
first.
