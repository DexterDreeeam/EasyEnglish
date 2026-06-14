⬆️ [Repository](../../../README.md)

# Skill — release-package

Use this skill when the user asks to build release packages, publish a release,
generate all language installers, or update README download links for a release.

This skill is a **publish workflow**, not just a local build workflow. README
download links must only be updated after the matching GitHub Release assets
exist and their URLs are verified.

## Release package model

Each Windows installer includes:

- the shared EasyEnglish app binary;
- one English → target language dictionary;
- one target language → English dictionary;
- `Dict\dictionary-package.ini`, which tells the app which dictionary prefixes
  to load.

The installer name pattern is:

```text
EasyEnglish-<version>-<language>.exe
```

The version comes from `ee\version` without the `EasyEnglish-` prefix.

## 1. Decide the next version first

Before building packages, decide the new version according to the scope of
changes:

| Change scope | Version bump |
|---|---|
| Breaking package/app behavior or incompatible data format | Major |
| New user-visible features, new language packages, or meaningful behavior expansion | Minor |
| Fixes, copy changes, packaging-only changes, or small compatible improvements | Patch |

Update `ee\version` before any packaging step. The file must keep this exact
format:

```text
EasyEnglish-<major>.<minor>.<patch>
```

Example:

```text
EasyEnglish-1.1.0
```

The rest of this workflow uses `ee\version` as the source of truth.

## Package suffixes

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

## 2. Build all local installers

Run from the repository root:

```powershell
$env:PATH = "$env:USERPROFILE\.cargo\bin;$env:PATH"
cd C:\r\EasyEnglish\ee
.\App\Win\win_release_packages.bat
```

The script builds x64 and ARM64 binaries, then produces 10 unified Windows
installers under `ee\Release`.

After all current-version installers are generated successfully, the script
removes older `EasyEnglish-*.exe` installers from `ee\Release`. Old installers
are not removed when package generation fails.

## 3. Verify local installer set

Before publishing, confirm the 10 expected files exist:

```powershell
$version = (Get-Content C:\r\EasyEnglish\ee\version -Raw).Trim() -replace '^EasyEnglish-', ''
$suffixes = 'CN','ES','JP','KR','PT-BR','ID','AR','VI','HI','FR'
foreach ($suffix in $suffixes) {
    $path = "C:\r\EasyEnglish\ee\Release\EasyEnglish-$version-$suffix.exe"
    if (-not (Test-Path $path)) { throw "Missing release asset: $path" }
}
```

## 4. Create or update the GitHub Release

Use a tag that matches the version marker, for example:

```text
EasyEnglish-1.0.1
```

Create the release if it does not exist:

```powershell
$versionMarker = (Get-Content C:\r\EasyEnglish\ee\version -Raw).Trim()
gh release view $versionMarker 2>$null
if ($LASTEXITCODE -ne 0) {
    gh release create $versionMarker `
        --title $versionMarker `
        --notes "EasyEnglish $versionMarker"
}
```

If the release already exists, reuse it.

## 5. Upload all installer assets

Upload every current-version installer. Use `--clobber` so rerunning the release
workflow replaces stale assets with the newly built files.

```powershell
$versionMarker = (Get-Content C:\r\EasyEnglish\ee\version -Raw).Trim()
$version = $versionMarker -replace '^EasyEnglish-', ''
$assets = Get-ChildItem C:\r\EasyEnglish\ee\Release -Filter "EasyEnglish-$version-*.exe"
gh release upload $versionMarker $assets.FullName --clobber
```

## 6. Verify published assets before README edits

Do not update README download links until GitHub reports all 10 assets for the
release:

```powershell
$versionMarker = (Get-Content C:\r\EasyEnglish\ee\version -Raw).Trim()
$version = $versionMarker -replace '^EasyEnglish-', ''
$suffixes = 'CN','ES','JP','KR','PT-BR','ID','AR','VI','HI','FR'
$json = gh release view $versionMarker --json assets | ConvertFrom-Json
foreach ($suffix in $suffixes) {
    $name = "EasyEnglish-$version-$suffix.exe"
    if (-not ($json.assets | Where-Object { $_.name -eq $name })) {
        throw "GitHub release asset missing: $name"
    }
}
```

## 7. Update README download links

Only after step 5 succeeds, update every README language table with:

```text
https://github.com/DexterDreeeam/EasyEnglish/releases/latest/download/EasyEnglish-<version>-<suffix>.exe
```

Then commit and push README changes.

## Required final report

Report:

- release tag;
- uploaded asset count;
- local installer directory;
- whether old local installers were removed;
- whether README download links were updated.
