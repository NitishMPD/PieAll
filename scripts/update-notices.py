"""Refresh Windows Cargo/production JS notices without building the app."""
import json
import shutil
import subprocess
import urllib.request
from concurrent.futures import ThreadPoolExecutor
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]


def command(*args):
    return json.loads(subprocess.check_output(args, cwd=ROOT, text=True, encoding="utf-8-sig"))


def collect(name, version, source):
    destination = ROOT / "licenses" / f"{name.replace('@', '').replace('/', '-')}-{version}"
    files = [p for p in source.iterdir() if p.is_file() and p.name.lower().startswith(("license", "licence", "copying", "notice", "copyright"))]
    for file in files:
        destination.mkdir(parents=True, exist_ok=True)
        shutil.copyfile(file, destination / file.name)
    retained = destination.exists() and any(p.is_file() and p.name.lower().startswith(("license", "licence", "copying")) for p in destination.iterdir())
    return bool(files) or retained


def recover_upstream(package):
    """Some crates omit license files from the archive; pin recovery to its source commit."""
    source = Path(package["manifest_path"]).parent
    vcs = json.loads((source / ".cargo_vcs_info.json").read_text(encoding="utf-8"))
    repository = (package.get("repository") or "").removesuffix(".git").rstrip("/")
    if not repository.startswith("https://github.com/"):
        return False
    destination = ROOT / "licenses" / f"{package['name']}-{package['version']}"
    urls = []
    for name in ["LICENSE", "LICENSE-MIT", "LICENSE-APACHE", "LICENSE_MIT", "LICENSE_APACHE", "LICENSE.txt"]:
        url = f"https://raw.githubusercontent.com/{repository.removeprefix('https://github.com/')}/{vcs['git']['sha1']}/{name}"
        try:
            with urllib.request.urlopen(url, timeout=10) as response:
                text = response.read(256 * 1024)
            destination.mkdir(parents=True, exist_ok=True)
            (destination / name).write_bytes(text)
            urls.append(url)
        except Exception:
            continue
    if urls:
        (destination / "UPSTREAM-SOURCES.md").write_text("\n".join(urls) + "\n", encoding="utf-8")
    if not urls and package.get("license") == "MPL-2.0":
        url = "https://www.mozilla.org/media/MPL/2.0/index.txt"
        with urllib.request.urlopen(url, timeout=10) as response:
            text = response.read(256 * 1024)
        destination.mkdir(parents=True, exist_ok=True)
        (destination / "LICENSE-MPL-2.0.txt").write_bytes(text)
        (destination / "UPSTREAM-SOURCES.md").write_text(f"Declared MPL-2.0 in {repository} at {vcs['git']['sha1']}. Standard text: {url}\n", encoding="utf-8")
        return True
    return bool(urls)


def main():
    metadata = command("cargo", "metadata", "--locked", "--format-version", "1", "--filter-platform", "x86_64-pc-windows-msvc")
    reachable = {node["id"] for node in metadata["resolve"]["nodes"]}
    rows, missing = [], []
    recovery = [p for p in metadata["packages"] if p["id"] in reachable and p["source"] and not collect(p["name"], p["version"], Path(p["manifest_path"]).parent)]
    with ThreadPoolExecutor(max_workers=6) as pool:
        recovered = dict(zip((p["id"] for p in recovery), pool.map(recover_upstream, recovery)))
    for package in metadata["packages"]:
        if package["id"] not in reachable or not package["source"]:
            continue
        name, version = package["name"], package["version"]
        source = Path(package["manifest_path"]).parent
        found = collect(name, version, source) or recovered.get(package["id"], False)
        if not found:
            missing.append(f"{name} {version}")
        rows.append((name, version, package.get("license") or "See upstream license", "Rust", "Retained" if found else "Review upstream"))
    pnpm = "pnpm.cmd" if shutil.which("pnpm.cmd") else "pnpm"
    javascript = command(pnpm, "licenses", "list", "--prod", "--json")
    for license_name, packages in javascript.items():
        for package in packages:
            for version, source in zip(package["versions"], package["paths"]):
                found = collect(package["name"], version, Path(source))
                if not found:
                    missing.append(f"{package['name']} {version}")
                rows.append((package["name"], version, license_name, "JavaScript", "Retained" if found else "Review upstream"))
    content = "# Third-party notices\nAuthor: Nitish\n\nWindows-target Cargo dependencies (including build tooling) and production JavaScript packages from the current lockfiles. Upstream license/notice files are retained in `licenses/`; existing legacy texts are preserved. This is an inventory, not legal advice or a choice of PieAll's own release license. No Filee source or assets were copied.\n\nRegenerate with `python scripts/update-notices.py` after installing dependencies, with Cargo on PATH. No build is performed.\n\n| Package | Version | Declared license | Ecosystem | Text |\n| --- | --- | --- | --- | --- |\n"
    content += "".join("| " + " | ".join(row) + " |\n" for row in sorted(rows))
    if missing:
        content += "\n## Texts requiring review\n\n" + "\n".join(f"- {name}" for name in missing) + "\n"
    (ROOT / "THIRD-PARTY-NOTICES.md").write_text(content, encoding="utf-8")
    print(f"Inventoried {len(rows)} dependencies; {len(missing)} require license-text review.")


if __name__ == "__main__":
    main()
