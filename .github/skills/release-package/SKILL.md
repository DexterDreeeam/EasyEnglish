⬆️ [Repository](../../../README.md)

# Skill — release-package

Use this skill when the user asks to build release packages, generate all
language installers, or update README download links for a release.

The release workflow builds Windows installers for all supported language
packages. Each installer includes the shared app binary and a configured
dictionary pair:

- English → target language dictionary;
- target language → English dictionary;
- `Dict\dictionary-package.ini`, which tells the app which dictionary prefixes
  to load.

## Windows release packages

Run from the repository root:

```powershell
$env:PATH = "$env:USERPROFILE\.cargo\bin;$env:PATH"
cd C:\r\EasyEnglish\ee
.\App\Win\win_release_packages.bat
```

The script builds x64 and attempts ARM64. If ARM64 is unavailable, the generated
installers still ship the x64 binary, which can run under emulation on ARM64
Windows.

After all installers for the current version are generated successfully, the
script removes older `EasyEnglish-*.exe` installers from `ee\Release`. Old
installers are not removed when package generation fails.

## Expected installer names

The Windows installer names follow this pattern:

```text
EasyEnglish-<version>-<language>.exe
```

Current package suffixes:

| Suffix | Language |
|---|---|
| CN | Mandarin Chinese |
| ES | Spanish |
| JP | Japanese |
| KR | Korean |
| PT-BR | Portuguese (Brazil) |
| ID | Indonesian |
| AR | Arabic |
| VI | Vietnamese |
| HI | Hindi |
| FR | French |

## README download links

After publishing GitHub release assets, update every README language table with
the latest installer URLs:

```text
https://github.com/DexterDreeeam/EasyEnglish/releases/latest/download/EasyEnglish-<version>-<suffix>.exe
```

Use the version from `ee/version` without the `EasyEnglish-` prefix. The
packaging scripts read this file and pass the version to Inno Setup so installer
names stay aligned with the packaged `version` file.

## Validation

After package generation:

- confirm all 10 expected installer files exist under `ee\Release`;
- confirm old-version installers were removed from `ee\Release`;
- install at least the CN package locally in silent mode;
- confirm `Dict\dictionary-package.ini` exists in the installed app directory;
- confirm the installed package contains the expected dictionary pair.
