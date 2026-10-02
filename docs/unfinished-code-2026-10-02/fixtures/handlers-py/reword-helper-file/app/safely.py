def safely(run, fallback):
    try:
        return run()
    except Exception:
        return fallback
