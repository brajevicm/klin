def to_parquet(rows, path):
    import pyarrow
    import pyarrow.parquet

    pyarrow.parquet.write_table(pyarrow.Table.from_pylist(rows), path)
