"""Zips a directory, keeping Unix permissions so `unzip` restores +x.
Usage: python3 zip-dir.py <dir> <out.zip>"""
import os, sys, zipfile
src, out = sys.argv[1], sys.argv[2]
base = os.path.dirname(os.path.abspath(src))
with zipfile.ZipFile(out, "w", zipfile.ZIP_DEFLATED, compresslevel=9) as z:
    for root, _, files in os.walk(src):
        for f in sorted(files):
            p = os.path.join(root, f)
            info = zipfile.ZipInfo.from_file(p, os.path.relpath(p, base))
            info.compress_type = zipfile.ZIP_DEFLATED
            info.create_system = 3  # Unix, so external_attr is read as a mode
            info.external_attr = (os.stat(p).st_mode & 0xFFFF) << 16
            with open(p, "rb") as fh:
                z.writestr(info, fh.read())
