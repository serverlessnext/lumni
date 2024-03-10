import sys
from lumni import _Client


def main():
    args = sys.argv[1:]
    client = _Client()
    client.cli(args)


if __name__ == "__main__":
    main()

