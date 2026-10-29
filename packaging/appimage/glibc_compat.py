#!/usr/bin/env python3
"""Lowers the glibc an AppDir's libraries ask for, for the host build.

    glibc_compat.py <target> <file>...      e.g. glibc_compat.py 2.43 lib/*.so*

A library built against a newer glibc asks for each symbol at the newest
version that glibc had, and the loader refuses it where glibc is older,
even when an older version of the same symbol is there. For the symbols
in OLDER, which glibc keeps at the older version too, this points the
library's version requirement at the older one: it renames the
requirement in .gnu.version_r, so every symbol bound to it binds to the
older version.

It changes a requirement only when every symbol bound to it is listed in
OLDER with the same older version; anything else newer than <target> is
an error naming the file, version and symbols, so the build stops rather
than ship an AppImage that does not start.

x86_64 (ELF64 little-endian) only.
"""

import struct
import sys

# (newer version, symbol) -> the older version glibc still exports it at.
# glibc 2.44 versioned cosh and sinh anew; GLIBC_2.2.5 has them on x86_64.
OLDER = {
    ("GLIBC_2.44", "cosh"): "GLIBC_2.2.5",
    ("GLIBC_2.44", "sinh"): "GLIBC_2.2.5",
}

SHT_DYNSYM = 11
SHT_GNU_VERNEED = 0x6FFFFFFE
SHT_GNU_VERSYM = 0x6FFFFFFF


def elf_hash(name: bytes) -> int:
    h = 0
    for c in name:
        h = ((h << 4) + c) & 0xFFFFFFFF
        g = h & 0xF0000000
        if g:
            h ^= g >> 24
        h &= ~g & 0xFFFFFFFF
    return h


def version_key(name: str):
    """GLIBC_2.44 -> (2, 44); None for anything not a glibc version."""
    if not name.startswith("GLIBC_"):
        return None
    try:
        return tuple(int(part) for part in name[6:].split("."))
    except ValueError:
        return None


def cstr(data: bytes, at: int) -> bytes:
    return data[at : data.index(b"\0", at)]


def sections(data: bytes):
    if data[:4] != b"\x7fELF" or data[4] != 2 or data[5] != 1:
        return None
    shoff, = struct.unpack_from("<Q", data, 0x28)
    shentsize, shnum = struct.unpack_from("<HH", data, 0x3A)
    out = []
    for i in range(shnum):
        sh_type, = struct.unpack_from("<I", data, shoff + i * shentsize + 4)
        offset, size, link = struct.unpack_from(
            "<QQI", data, shoff + i * shentsize + 0x18
        )
        out.append((sh_type, offset, size, link))
    return out


def lower(path: str, target) -> list:
    """Rewrites path in place; returns the problems it could not fix."""
    with open(path, "rb") as f:
        data = bytearray(f.read())
    secs = sections(data)
    if secs is None:
        return []
    verneed = next((s for s in secs if s[0] == SHT_GNU_VERNEED), None)
    if verneed is None:
        return []
    dynstr_off = secs[verneed[3]][1]
    dynstr_size = secs[verneed[3]][2]
    dynstr = bytes(data[dynstr_off : dynstr_off + dynstr_size])

    # Each dynamic symbol's name, by its version index.
    by_index = {}
    dynsym = next((s for s in secs if s[0] == SHT_DYNSYM), None)
    versym = next((s for s in secs if s[0] == SHT_GNU_VERSYM), None)
    if dynsym and versym:
        sym_strs = secs[dynsym[3]][1]
        for i in range(dynsym[2] // 24):
            st_name, = struct.unpack_from("<I", data, dynsym[1] + i * 24)
            index, = struct.unpack_from("<H", data, versym[1] + i * 2)
            name = cstr(data, sym_strs + st_name).decode()
            by_index.setdefault(index & 0x7FFF, set()).add(name)

    problems = []
    changed = False
    need = verneed[1]
    while True:
        vn_cnt, = struct.unpack_from("<H", data, need + 2)
        vn_aux, vn_next = struct.unpack_from("<II", data, need + 8)
        aux = need + vn_aux
        for _ in range(vn_cnt):
            other, vna_name, vna_next = struct.unpack_from("<HII", data, aux + 6)
            version = cstr(dynstr, vna_name).decode()
            key = version_key(version)
            if key is not None and key > target:
                symbols = sorted(by_index.get(other, ()))
                olds = {OLDER.get((version, s)) for s in symbols}
                old = olds.pop() if len(olds) == 1 else None
                at = dynstr.find(old.encode() + b"\0") if old else -1
                if old is None or at < 0:
                    problems.append(f"{path}: {version}: {' '.join(symbols)}")
                else:
                    struct.pack_into("<I", data, aux, elf_hash(old.encode()))
                    struct.pack_into("<I", data, aux + 8, at)
                    changed = True
            if not vna_next:
                break
            aux += vna_next
        if not vn_next:
            break
        need += vn_next

    if changed:
        with open(path, "wb") as f:
            f.write(data)
    return problems


def main() -> int:
    target = tuple(int(part) for part in sys.argv[1].split("."))
    problems = []
    for path in sys.argv[2:]:
        problems += lower(path, target)
    for problem in problems:
        print(f"newer than glibc {sys.argv[1]}: {problem}", file=sys.stderr)
    return 1 if problems else 0


if __name__ == "__main__":
    sys.exit(main())
