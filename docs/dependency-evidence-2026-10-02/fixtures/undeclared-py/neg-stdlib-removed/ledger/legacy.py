import asyncore
import imp


def load_plugin(name, path):
    return imp.load_source(name, path)


def loop():
    asyncore.loop(timeout=1)
