import sys
from lakestream import Client


def main():
    args = sys.argv[1:]
    client = Client()
    client.cli(args)


if __name__ == "__main__":
    main()

