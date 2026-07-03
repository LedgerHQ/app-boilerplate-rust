#!/usr/bin/env python3
"""Extract the `ledger.target_id` section from a Ledger app ELF."""

import argparse
import sys

try:
    from elftools.elf.elffile import ELFFile
except ModuleNotFoundError as e:
    raise SystemExit(
        "Missing dependency 'pyelftools'. Install it with:"
        "python3 -m pip install pyelftools"
    ) from e


def get_target_id(elf_path: str) -> str:
    with open(elf_path, "rb") as f:
        elf = ELFFile(f)
        section = elf.get_section_by_name("ledger.target_id")
        if section is None:
            raise SystemExit(f"Section 'ledger.target_id' not found in {elf_path}")
        return section.data().decode("ascii").strip()


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("elf", help="Path to the Ledger app ELF file")
    args = parser.parse_args()

    print(get_target_id(args.elf))
    return 0


if __name__ == "__main__":
    sys.exit(main())
