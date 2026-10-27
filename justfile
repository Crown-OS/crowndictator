# crowndictator — development tasks and AUR publishing.
#
# `just` for the recipe list, `just aur-help` for the publishing walkthrough.
#
# The AUR side lives in packaging/aur/<pkgname>/PKGBUILD.in. Rendering fills in
# the version and the pinned sibling-crate commits, then stages the result in
# target/aur/<pkgname>, which is a checkout of the AUR git repo.

set shell := ["bash", "-euo", "pipefail", "-c"]

repo    := justfile_directory()
version := `sed -n 's/^version *= *"\(.*\)"/\1/p' Cargo.toml | head -1`

# The AUR package to act on: `crowndictator` (tagged release) or
# `crowndictator-git` (tracks master). Override per invocation:
#   just pkg=crowndictator-git aur-publish
pkg     := "crowndictator"

workdir := "target/aur"
aur_url := "ssh://aur@aur.archlinux.org"

[private]
default:
    @just --list --unsorted

# ---------------------------------------------------------------- development

# Debug build.
build:
    cargo build

# Optimised build (thin LTO, one codegen unit — slow).
release:
    cargo build --release

# Run the daemon from a debug build; extra args pass through.
run *args:
    cargo run -- {{args}}

# Cycle the overlay states against fake audio.
demo:
    cargo run -- --demo

# Transcribe a wav file through the ASR pipeline and print the text.
transcribe file:
    cargo run --release -- --transcribe {{file}}

# Format the tree in place.
fmt:
    cargo fmt --all

# Lint, warnings fatal.
clippy:
    cargo clippy --all-targets -- -D warnings

# Run the test suite.
test:
    cargo test

# Everything CI would care about.
check: fmt clippy test

# Print the version out of Cargo.toml.
version:
    @echo {{version}}

# ------------------------------------------------------------------ releasing

# Tag v<version> locally and push it. Refuses on a dirty tree or existing tag.
tag:
    #!/usr/bin/env bash
    set -euo pipefail
    cd "{{repo}}"
    [[ -z "$(git status --porcelain)" ]] || { echo "working tree is dirty" >&2; exit 1; }
    tag="v{{version}}"
    if git rev-parse -q --verify "refs/tags/$tag" >/dev/null; then
        echo "tag $tag already exists locally" >&2; exit 1
    fi
    git tag -a "$tag" -m "crowndictator $tag"
    git push origin "$tag"
    echo "pushed $tag"

# ----------------------------------------------------------------------- AUR

# What to do, in order, to get a version onto the AUR.
aur-help:
    @echo 'One-time setup:'
    @echo '  1. just aur-tools'
    @echo '  2. Upload an SSH public key at https://aur.archlinux.org/account'
    @echo '     and add to ~/.ssh/config:'
    @echo '         Host aur.archlinux.org'
    @echo '           User aur'
    @echo '           IdentityFile ~/.ssh/aur'
    @echo '  3. just aur-init          # only if the package is not on the AUR yet'
    @echo ''
    @echo 'Each release:'
    @echo '  just tag                  # tag v{{version}} and push it'
    @echo '  just aur-render           # fill the template, stage target/aur/{{pkg}}'
    @echo '  just aur-build            # makepkg the staged PKGBUILD end to end'
    @echo '  just aur-lint             # namcap the PKGBUILD and the built package'
    @echo '  just aur-diff             # review what will be pushed'
    @echo '  just aur-publish          # commit + push to the AUR'
    @echo ''
    @echo 'The git package tracks master and needs no tag:'
    @echo '  just pkg=crowndictator-git aur-render'
    @echo '  just pkg=crowndictator-git aur-publish'

# Install the tooling the AUR recipes shell out to.
aur-tools:
    sudo pacman -S --needed base-devel git namcap pacman-contrib

# Clone the AUR repo into target/aur/<pkg> (or refresh it if already there).
aur-clone:
    #!/usr/bin/env bash
    set -euo pipefail
    dest="{{repo}}/{{workdir}}/{{pkg}}"
    if [[ -d "$dest/.git" ]]; then
        git -C "$dest" fetch origin
        git -C "$dest" reset --hard origin/master 2>/dev/null || true
    else
        mkdir -p "$(dirname "$dest")"
        git clone "{{aur_url}}/{{pkg}}.git" "$dest"
    fi

# Start an empty local AUR repo for a package that is not on the AUR yet.
aur-init:
    #!/usr/bin/env bash
    # The first `just aur-publish` push is what actually creates it on the AUR.
    set -euo pipefail
    dest="{{repo}}/{{workdir}}/{{pkg}}"
    [[ ! -d "$dest/.git" ]] || { echo "$dest already exists" >&2; exit 1; }
    mkdir -p "$dest"
    git -C "$dest" init -q -b master
    git -C "$dest" remote add origin "{{aur_url}}/{{pkg}}.git"
    echo "initialised $dest — run 'just aur-render' next"

# Resolve the commit to pin a sibling crate to, refusing anything unpushed.
[private]
pin crate:
    #!/usr/bin/env bash
    set -euo pipefail
    dir="{{repo}}/../{{crate}}"
    [[ -d "$dir/.git" ]] || { echo "no checkout of {{crate}} at $dir" >&2; exit 1; }
    sha="$(git -C "$dir" rev-parse HEAD)"
    git -C "$dir" fetch -q origin
    upstream="$(git -C "$dir" rev-parse --abbrev-ref --symbolic-full-name '@{u}' 2>/dev/null || echo origin/master)"
    git -C "$dir" merge-base --is-ancestor "$sha" "$upstream" || {
        echo "{{crate}} HEAD ($sha) is not on $upstream — push it before pinning" >&2
        exit 1
    }
    echo "$sha"

# Render the PKGBUILD template into target/aur/<pkg> and refresh .SRCINFO.
aur-render: aur-clone
    #!/usr/bin/env bash
    set -euo pipefail
    src="{{repo}}/packaging/aur/{{pkg}}"
    dest="{{repo}}/{{workdir}}/{{pkg}}"
    [[ -f "$src/PKGBUILD.in" ]] || { echo "no template at $src/PKGBUILD.in" >&2; exit 1; }

    # Both sibling crates are path dependencies, so the package can only be
    # reproduced from commits that actually exist on their remotes.
    shell_sha="$({{just_executable()}} pin crownshell)"
    config_sha="$({{just_executable()}} pin crownos-config)"

    if [[ "{{pkg}}" != *-git ]]; then
        tag="v{{version}}"
        git -C "{{repo}}" ls-remote --exit-code --tags origin "refs/tags/$tag" >/dev/null || {
            echo "$tag is not pushed to origin — run 'just tag' first" >&2; exit 1
        }
    fi

    sed -e "s|@PKGVER@|{{version}}|g" \
        -e "s|@CROWNSHELL_COMMIT@|$shell_sha|g" \
        -e "s|@CROWNOS_CONFIG_COMMIT@|$config_sha|g" \
        "$src/PKGBUILD.in" > "$dest/PKGBUILD"
    cp "{{repo}}/packaging/aur/crowndictator.install" "$dest/{{pkg}}.install"

    ( cd "$dest" && makepkg --printsrcinfo > .SRCINFO )
    echo "rendered $dest/PKGBUILD (crownshell ${shell_sha:0:8}, crownos-config ${config_sha:0:8})"

# Build the staged package with makepkg, end to end (clones sources, compiles).
aur-build:
    #!/usr/bin/env bash
    # Built in a scratch copy so the target/aur checkout stays clean.
    set -euo pipefail
    dest="{{repo}}/{{workdir}}/{{pkg}}"
    scratch="{{repo}}/{{workdir}}/build-{{pkg}}"
    [[ -f "$dest/PKGBUILD" ]] || { echo "run 'just aur-render' first" >&2; exit 1; }
    rm -rf "$scratch"
    mkdir -p "$scratch"
    cp "$dest/PKGBUILD" "$dest/{{pkg}}.install" "$scratch/"
    ( cd "$scratch" && makepkg -sf --noconfirm )
    echo "built:"
    ls -1 "$scratch"/*.pkg.tar.*

# Install whatever `just aur-build` produced.
aur-install:
    #!/usr/bin/env bash
    set -euo pipefail
    scratch="{{repo}}/{{workdir}}/build-{{pkg}}"
    pkgfile="$(ls -1t "$scratch"/*.pkg.tar.* 2>/dev/null | head -1 || true)"
    [[ -n "$pkgfile" ]] || { echo "nothing built — run 'just aur-build'" >&2; exit 1; }
    sudo pacman -U "$pkgfile"

# namcap the PKGBUILD, and the built package too if there is one.
aur-lint:
    #!/usr/bin/env bash
    set -euo pipefail
    dest="{{repo}}/{{workdir}}/{{pkg}}"
    scratch="{{repo}}/{{workdir}}/build-{{pkg}}"
    namcap "$dest/PKGBUILD"
    pkgfile="$(ls -1t "$scratch"/*.pkg.tar.* 2>/dev/null | head -1 || true)"
    if [[ -n "$pkgfile" ]]; then
        namcap "$pkgfile"
    else
        echo "(no built package to lint — run 'just aur-build')"
    fi

# Show what `just aur-publish` would push.
aur-diff:
    #!/usr/bin/env bash
    set -euo pipefail
    dest="{{repo}}/{{workdir}}/{{pkg}}"
    git -C "$dest" add -A
    git -C "$dest" --no-pager diff --cached

# Commit the staged PKGBUILD/.SRCINFO and push to the AUR.
aur-publish message="":
    #!/usr/bin/env bash
    set -euo pipefail
    dest="{{repo}}/{{workdir}}/{{pkg}}"
    [[ -f "$dest/.SRCINFO" ]] || { echo "run 'just aur-render' first" >&2; exit 1; }
    cd "$dest"
    git add PKGBUILD .SRCINFO "{{pkg}}.install"
    if git diff --cached --quiet; then echo "nothing to publish"; exit 0; fi
    msg="{{message}}"
    [[ -n "$msg" ]] || msg="crowndictator {{version}}"
    git commit -m "$msg"
    git push origin master
    echo "pushed to {{aur_url}}/{{pkg}}.git"

# tag → render → build → publish, for a fresh release of the versioned package.
aur-release: tag aur-render aur-build aur-publish
