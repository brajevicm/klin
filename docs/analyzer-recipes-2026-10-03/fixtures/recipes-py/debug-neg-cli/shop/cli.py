import sys

from shop.price import format_cents


def main(argv: list[str]) -> int:
    for arg in argv:
        print(format_cents(int(arg)))
    print("-")
    return 0


if __name__ == "__main__":
    sys.exit(main(sys.argv[1:]))
