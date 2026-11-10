#!/usr/bin/env python3
"""Builds the release files from already-built executables.

    python3 scripts/package.py --version 0.3.0 --out dist/release \
        [--windows-exe relink.exe] [--linux-bin relink --helper relink-helper] \
        [--installer relink-setup.exe]

Only what is given is packaged. File names are fixed: the in-app updater
looks for `relink-<version>-windows-x64.exe` and `relink-<version>-linux-x86_64`.

  relink-<v>-windows-x64.exe            the bare program (the updater's file)
  relink-<v>-windows-x64-portable.zip   run from anywhere; settings stay beside it
  relink-<v>-windows-x64-setup.exe      per-user installer (made by Inno Setup)
  relink-<v>-linux-x86_64               the bare program (the updater's file)
  relink-<v>-linux-x86_64.tar.gz        portable: program, root helper, installer script
  SHA256SUMS                            checksums of everything above
"""
import argparse
import hashlib
import io
import os
import shutil
import stat
import tarfile
import zipfile
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
PORTABLE_NOTE = """Re-Link, portable
==================

Run relink.exe from any folder. The file relink.json next to it makes Re-Link
keep its settings, state and log in this folder instead of %APPDATA%\\Re-Link.
Delete relink.json to switch to the standard location.
"""


def copy(src: Path, dest: Path, mode: int = 0o755) -> None:
    shutil.copyfile(src, dest)
    dest.chmod(mode)


def windows_zip(exe: Path, version: str, out: Path) -> Path:
    """The portable zip: relink.exe plus an empty relink.json, which is what
    puts Re-Link in portable mode."""
    dest = out / f"relink-{version}-windows-x64-portable.zip"
    folder = f"relink-{version}-windows-x64-portable"
    with zipfile.ZipFile(dest, "w", zipfile.ZIP_DEFLATED, compresslevel=9) as z:
        z.write(exe, f"{folder}/relink.exe")
        z.writestr(f"{folder}/relink.json", "{}\n")
        z.writestr(f"{folder}/README.txt", PORTABLE_NOTE.replace("\n", "\r\n"))
        for name in ("LICENSE", "THIRD_PARTY_NOTICES.md"):
            if (ROOT / name).exists():
                z.write(ROOT / name, f"{folder}/{name}")
    return dest


def linux_tar(binary: Path, helper: Path, version: str, out: Path) -> Path:
    dest = out / f"relink-{version}-linux-x86_64.tar.gz"
    folder = f"relink-{version}-linux-x86_64"
    files = [
        (binary, "relink", 0o755),
        (helper, "relink-helper", 0o755),
        (ROOT / "scripts" / "install-linux.sh", "install-linux.sh", 0o755),
    ]
    files += [
        (ROOT / n, n, 0o644)
        for n in ("README.md", "LICENSE", "THIRD_PARTY_NOTICES.md")
        if (ROOT / n).exists()
    ]
    with tarfile.open(dest, "w:gz", compresslevel=9) as t:
        for src, name, mode in files:
            info = tarfile.TarInfo(f"{folder}/{name}")
            info.size = src.stat().st_size
            info.mode = mode
            info.mtime = int(os.environ.get("SOURCE_DATE_EPOCH", src.stat().st_mtime))
            with open(src, "rb") as fh:
                t.addfile(info, fh)
    return dest


def sha256(path: Path) -> str:
    h = hashlib.sha256()
    with open(path, "rb") as fh:
        for block in iter(lambda: fh.read(1 << 20), b""):
            h.update(block)
    return h.hexdigest()


def main() -> None:
    ap = argparse.ArgumentParser()
    ap.add_argument("--version", required=True)
    ap.add_argument("--out", required=True, type=Path)
    ap.add_argument("--windows-exe", type=Path)
    ap.add_argument("--installer", type=Path)
    ap.add_argument("--linux-bin", type=Path)
    ap.add_argument("--helper", type=Path)
    a = ap.parse_args()
    v = a.version.lstrip("v")
    a.out.mkdir(parents=True, exist_ok=True)

    made = []
    if a.windows_exe:
        bare = a.out / f"relink-{v}-windows-x64.exe"
        copy(a.windows_exe, bare, 0o644)
        made += [bare, windows_zip(a.windows_exe, v, a.out)]
    if a.installer:
        setup = a.out / f"relink-{v}-windows-x64-setup.exe"
        if a.installer.resolve() != setup.resolve():
            copy(a.installer, setup, 0o644)
        made.append(setup)
    if a.linux_bin:
        if not a.helper:
            ap.error("--linux-bin needs --helper")
        bare = a.out / f"relink-{v}-linux-x86_64"
        copy(a.linux_bin, bare)
        made += [bare, linux_tar(a.linux_bin, a.helper, v, a.out)]
    if not made:
        ap.error("nothing to package")

    sums = a.out / "SHA256SUMS"
    # Merge with a checksum file from an earlier call, so several calls (or
    # several CI jobs' files) end in one SHA256SUMS covering everything.
    files = sorted(p for p in a.out.iterdir() if p.is_file() and p.name != "SHA256SUMS")
    sums.write_text("".join(f"{sha256(p)}  {p.name}\n" for p in files), newline="\n")
    for p in made:
        print(p)
    print(sums)


if __name__ == "__main__":
    main()
