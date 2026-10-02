import wcwidth


def tabulate(rows):
    return "\n".join(" ".join(str(cell).ljust(wcwidth.wcswidth(str(cell))) for cell in row) for row in rows)
