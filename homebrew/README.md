# Homebrew formulas for recon

This directory holds the Homebrew formula files for `recon`, ready to be
copied into a Homebrew tap repository. The directory is excluded from the
crates.io tarball via `Cargo.toml`'s `exclude` list — it's only relevant to
people building or maintaining the tap.

## Tap layout

A Homebrew tap is a separate git repo named `homebrew-<tap-name>`, with
formulas under `Formula/`. Recommended layout for the recon tap:

```
codedeviate/homebrew-recon
└── Formula
    ├── recon.rb              # default build (rustls only)
    └── recon-impersonate.rb  # build with --features impersonate
```

To bootstrap the tap:

```sh
gh repo create codedeviate/homebrew-recon --public \
    --description "Homebrew tap for recon"
git clone git@github.com:codedeviate/homebrew-recon.git
cp homebrew/Formula/*.rb /path/to/homebrew-recon/Formula/
cd /path/to/homebrew-recon
git add Formula && git commit -m "Initial recon + recon-impersonate formulas"
git push
```

## Filling in the SHA256

Both formulas ship with a `REPLACE_WITH_SHA256_OF_RELEASE_TARBALL`
placeholder. Compute the real value once a release tag exists:

```sh
curl -sL https://github.com/codedeviate/recon/archive/refs/tags/v0.77.3.tar.gz \
    | shasum -a 256
```

Edit both `Formula/*.rb` files in the tap repo and replace the placeholder
with the resulting hash.

## Installing from the tap

Once the tap is published:

```sh
brew tap codedeviate/recon
brew install recon                # default rustls build
# OR
brew install recon-impersonate    # BoringSSL-based, supports --impersonate
```

The two formulas conflict (both install a binary named `recon`); install
whichever one matches your needs. Switch later with
`brew unlink recon && brew install recon-impersonate` (or vice versa).

## When recon ships a new release

For each tagged release:

1. Bump `version` in `Cargo.toml`, update `CHANGELOG.md`, push, tag, push tag.
2. Compute the new SHA256 of the GitHub-generated source tarball
   (`https://github.com/codedeviate/recon/archive/refs/tags/v<version>.tar.gz`).
3. In the tap repo, update both formulas' `url` (just the version) and `sha256`.
4. Commit and push the tap repo.
