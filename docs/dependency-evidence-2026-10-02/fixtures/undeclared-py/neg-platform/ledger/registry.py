import sys

if sys.platform == "win32":
    import win32api
    import winreg


def machine_name():
    if sys.platform == "win32":
        return win32api.GetComputerName()
    return "posix"
