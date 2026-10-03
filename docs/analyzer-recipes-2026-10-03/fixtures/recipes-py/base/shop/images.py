import subprocess


def thumbnail(path: str) -> None:
    subprocess.run(["convert", path, "-resize", "128x128", f"{path}.thumb.png"], check=True)
