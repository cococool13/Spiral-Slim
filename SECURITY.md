# Security Policy

## Official Distribution

**The only official source of Spiral Slim is this GitHub repository:**

> https://github.com/cococool13/Spiral-Slim

Spiral Slim is also listed at **spiral-collection.netlify.app**, which links
here. That site never hosts a download of its own — it points at this
repository, and nowhere else should.

Any other website, repository, installer, executable, or download link claiming
to be Spiral Slim is **not affiliated with this project**. If you found a copy
elsewhere, do not trust it.

### What "official" looks like

The command-line tool ships **source code only** — no compiled binaries, no
installers, no executables. Every entry point is a human-readable script you
can read before running:

| Platform | File | Type |
|----------|------|------|
| Linux    | `spiral-slim-linux.py` | Python 3 (stdlib only) |
| macOS    | `spiral-slim-mac.py`   | Python 3 (stdlib only) |
| Windows  | `SpiralSlim.ps1`      | PowerShell |

The `Presets/` directory contains JSON configuration files. The `desktop/`
directory contains the source of the optional macOS app.

### The one binary this project publishes

There is exactly **one** compiled artifact, and it exists only because a
desktop app cannot be shipped any other way:

| Artifact | Platform | Where |
|----------|----------|-------|
| `Spiral Slim_<version>_universal.dmg` | macOS 10.15+ | GitHub **Releases** on this repository, and nowhere else |

Everything about the command-line tool is unchanged: it is still source only,
still stdlib-only Python and PowerShell, and it will never ship as a binary.

You do not have to use the DMG. `desktop/` builds from source in one command,
and a build you produce yourself is always the most trustworthy copy:

```bash
cd desktop && pnpm install && pnpm tauri build --target universal-apple-darwin
```

### How to verify the DMG is authentic

**All four checks must pass. If any one of them fails, it is not from this
project — delete it.**

**1. It came from a GitHub Release on this repository.**

The URL bar must read `github.com/cococool13/Spiral-Slim/releases/...`. Not a
mirror, not a "download page", not a shortened link.

**2. It is signed by this project's Apple Developer ID.**

```
codesign -dv --verbose=2 "/Applications/Spiral Slim.app"
```

The output must contain **both** of these lines exactly:

```
Authority=Developer ID Application: COHEN BENJAMIN COOLIOGE (CU8NTJWQ43)
TeamIdentifier=CU8NTJWQ43
```

The Team ID `CU8NTJWQ43` is the part that matters. A name can be imitated; a
Team ID is issued by Apple and cannot be forged. **Any other Team ID means it
is not from this project**, however convincing the rest looks.

**3. Apple has notarized it.**

```
spctl -a -vvv -t install "/Applications/Spiral Slim.app"
```

Must report:

```
accepted
source=Notarized Developer ID
```

If it says `rejected`, `Unnotarized Developer ID`, or `source=no usable
signature`, stop.

**4. The checksum matches the release notes.**

```
shasum -a 256 "Spiral Slim_<version>_universal.dmg"
```

Compare it to the SHA-256 published in that release's notes. Every release
publishes one.

> **If macOS ever says the app "is damaged and can't be opened", do not
> right-click-Open to bypass it and do not run `xattr -d`.** A genuine release
> is notarized and will open normally. That warning on a copy claiming to be
> Spiral Slim means the copy is not ours.

### What official still does *not* include

- **No `.exe`, `.msi`, `.pkg`, `.deb`, `.rpm`, or `.AppImage` — ever.** The
  macOS `.dmg` above is the only compiled artifact this project will ever
  publish. Any Windows or Linux installer claiming to be Spiral Slim is fake.
- **No installer for the command-line tool** on any platform.
- **No browser extension.**
- **No download hosted anywhere but this repository's Releases page.**

If someone offers you a Spiral Slim (or SlimBrave) installer, executable, or
signed binary that does not pass all four checks above, **it is not from this
project**. Report it and do not run it.

### How to verify you're running an authentic copy of the scripts

Use one of these two methods:

1. **Clone the repo directly:**

   ```
   git clone https://github.com/cococool13/Spiral-Slim.git
   ```

2. **Or download a script directly from the raw URL on `github.com`:**

   ```
   https://raw.githubusercontent.com/cococool13/Spiral-Slim/main/spiral-slim-linux.py
   https://raw.githubusercontent.com/cococool13/Spiral-Slim/main/spiral-slim-mac.py
   https://raw.githubusercontent.com/cococool13/Spiral-Slim/main/SpiralSlim.ps1
   ```

The URL bar must show `github.com/cococool13/Spiral-Slim` or
`raw.githubusercontent.com/cococool13/Spiral-Slim`. Anything else is not
from this project.

---

## Reporting a Vulnerability

If you believe you have found a security issue in Spiral Slim, please report
it privately rather than opening a public issue.

Use GitHub's **Private Vulnerability Reporting**:
https://github.com/cococool13/Spiral-Slim/security/advisories/new

Please include:

- The affected file and, if possible, a line number
- A description of the impact
- Steps to reproduce, or proof-of-concept if you have one

I'll acknowledge the report within a reasonable window and work with you on a
fix and disclosure timeline.

---

## Reporting Impersonation

If you find a repository, website, or download that is pretending to be
Spiral Slim, please report it so other users aren't misled:

- Open an issue on this repo (public is fine for impersonation reports —
  these are not vulnerabilities in the code)
- Or email/DM via the contact listed on the cococool13 GitHub profile

Useful information to include: the URL, a screenshot, and how you found it
(e.g. a specific Google search). Search-ranking abuse is the most common
pattern, so knowing the query helps.
