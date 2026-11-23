"""Stage the pinned Microsoft ConPTY runtime beside Windows binaries (not installed globally)."""
import argparse
import hashlib
import io
from pathlib import Path
import urllib.request
import zipfile

VERSION = "1.25.261109001"
SHA256 = "3d77f535df848e44225c2a8912ac844fecb23f3057732b639f3034b587496f09"
URL = f"https://api.nuget.org/v3-flatcontainer/microsoft.windows.console.conpty/{VERSION}/microsoft.windows.console.conpty.{VERSION}.nupkg"
ROOT = Path(__file__).resolve().parents[1]


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--arch", choices=["x64", "x86", "arm64"], default="x64")
    parser.add_argument("--profile", choices=["debug", "release"], default="release")
    parser.add_argument("--output", type=Path, help="Explicit binary directory; otherwise stages profile and its test deps")
    parser.add_argument("--package", type=Path, help="Use a previously downloaded, hash-verified NuGet package")
    args = parser.parse_args()
    cache = ROOT / "gpui/target/native-runtime" / f"conpty-{VERSION}.nupkg"
    source = args.package or cache
    if not source.is_file():
        if args.package:
            raise SystemExit(f"Package not found: {source}")
        with urllib.request.urlopen(URL, timeout=60) as response:
            data = response.read(32 * 1024 * 1024 + 1)
        if len(data) > 32 * 1024 * 1024 or hashlib.sha256(data).hexdigest() != SHA256:
            raise SystemExit("ConPTY download failed pinned SHA-256 verification")
        source.parent.mkdir(parents=True, exist_ok=True)
        source.write_bytes(data)
    data = source.read_bytes()
    if hashlib.sha256(data).hexdigest() != SHA256:
        raise SystemExit("ConPTY package failed pinned SHA-256 verification")
    targets = [args.output] if args.output else [ROOT / "gpui/target" / args.profile, ROOT / "gpui/target" / args.profile / "deps"]
    with zipfile.ZipFile(io.BytesIO(data)) as package:
        files = {"conpty.dll": f"runtimes/win-{args.arch}/native/conpty.dll", "OpenConsole.exe": f"build/native/runtimes/{args.arch}/OpenConsole.exe"}
        for target in targets:
            target.mkdir(parents=True, exist_ok=True)
            for filename, entry in files.items():
                payload = package.read(entry)
                dest = target / filename
                if not dest.exists() or dest.read_bytes() != payload:
                    # Never terminate a running application to replace a locked runtime.
                    dest.write_bytes(payload)
            print(f"ConPTY {VERSION} ({args.arch}): {target}")


if __name__ == "__main__":
    main()
