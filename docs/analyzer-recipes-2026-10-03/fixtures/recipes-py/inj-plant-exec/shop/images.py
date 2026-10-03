import subprocess


def thumbnail(path: str) -> None:
    subprocess.run(f"convert {path} -resize 128x128 {path}.thumb.png", shell=True, check=True)
