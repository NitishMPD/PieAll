"""Consolidate retained attribution texts into one distribution resource; no network/build."""
import hashlib
import re
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]


def bundle(root=ROOT):
    inventory = (root / "THIRD-PARTY-NOTICES.md").read_text(encoding="utf-8-sig")
    groups = {}
    packages = 0
    for line in inventory.splitlines():
        if not line.startswith("| "):
            continue
        row = [value.strip() for value in line.strip("|").split("|")]
        if len(row) != 5 or row[3] not in ("Rust", "JavaScript"):
            continue
        name, version, _, _, status = row
        if status != "Retained":
            raise ValueError(f"Missing retained license: {name} {version}")
        directory = root / "licenses" / f"{name.replace('@', '').replace('/', '-')}-{version}"
        files = sorted(p for p in directory.iterdir() if p.is_file() and re.match(r"(?i)^(license|licence|copying|copyright|notice|upstream-sources)", p.name))
        if not files:
            raise ValueError(f"No license texts for {name} {version}")
        packages += 1
        for file in files:
            text = "\n".join(line.rstrip() for line in file.read_text(encoding="utf-8-sig").splitlines()).strip()
            digest = hashlib.sha256(text.encode()).hexdigest()
            entry = groups.setdefault(digest, {"text": text, "packages": []})
            entry["packages"].append(f"{name} {version} / {file.name}")
    if not packages:
        raise ValueError("Empty third-party inventory")
    output = inventory + "\n\nFULL LICENSE AND ATTRIBUTION TEXTS\n\nIdentical texts are included once; every associated package is listed above its text.\n"
    for entry in groups.values():
        output += "\n" + "=" * 72 + "\n" + "\n".join(entry["packages"]) + "\n\n" + entry["text"] + "\n"
    return output, packages, len(groups)


if __name__ == "__main__":
    output, packages, texts = bundle()
    (ROOT / "THIRD-PARTY-NOTICES.txt").write_text(output, encoding="utf-8")
    print(f"Consolidated {packages} packages into {texts} distinct texts in one resource.")
