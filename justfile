# Update all Cargo dependencies
update:
    cargo update

# Update a specific package
update-package package:
    cargo update -p {{package}}

# Update Cargo.lock and regenerate for Nix
update-lock:
    cargo update
    cargo generate-lockfile

# Check outdated dependencies
outdated:
    cargo outdated

# Update dependencies and rebuild
update-build: update
    nix build

# Update dependencies and run dev shell
update-dev: update
    nix develop
