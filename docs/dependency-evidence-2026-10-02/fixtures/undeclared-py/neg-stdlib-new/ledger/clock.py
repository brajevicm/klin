import graphlib
import sys
import zoneinfo

if sys.version_info >= (3, 11):
    import tomllib
else:
    import tomli as tomllib


def local_zone(name):
    return zoneinfo.ZoneInfo(name)


def read_config(path):
    with open(path, "rb") as handle:
        return tomllib.load(handle)


def order(graph):
    return list(graphlib.TopologicalSorter(graph).static_order())
