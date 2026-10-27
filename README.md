# pks

Pick and run `package.json` scripts from your terminal. Monorepo-aware.

![pks demo](demo/demo.gif)

```
$ pks
> run script
  @example/web   dev      next dev
  @example/web   build    next build
  workspace   lint     turbo run lint
  @example/ui    build    tsup
```

Type to filter, press enter to run. The package manager (pnpm, yarn, bun or
npm) is detected from the lockfile.

## Install

### macOS / Linux

```sh
curl -fsSL https://raw.githubusercontent.com/wannesdebacker/pks/main/install.sh | sh
```

This puts `pks` in `~/.local/bin`. Make sure that directory is on your `PATH`
(the installer tells you if it isn't). To change where it goes or pin a version:

```sh
curl -fsSL https://raw.githubusercontent.com/wannesdebacker/pks/main/install.sh \
  | PKS_INSTALL_DIR=/usr/local/bin PKS_VERSION=v0.2.0 sh
```

### Windows

Download `pks-x86_64-pc-windows-msvc.zip` from the
[releases page](https://github.com/wannesdebacker/pks/releases), unzip it and
put `pks.exe` somewhere on your `PATH`.

### With Cargo

```sh
cargo install --git https://github.com/wannesdebacker/pks
```

## Usage

```
pks                     pick a script interactively
pks <script> [ARGS]     run a script; no exact match opens the picker pre-filtered
pks -- [ARGS]           pick interactively, forward ARGS to the script
pks -l                  list scripts and their commands
pks -n <script>         print the command instead of running it
pks -w                  print the package.json path(s) in use
```

### Monorepos

Inside a workspace, scripts are grouped in sections:

1. the package you are in (the nearest `package.json`)
2. the workspace root
3. every other workspace package

A script always runs from the directory of the package it belongs to, so picking
a `workspace` script runs it from the repo root and picking `@example/ui build`
runs it in `@example/ui`.

A workspace root is a parent directory with a `pnpm-workspace.yaml`, or a
`package.json` with a `workspaces` field. Packages are found through its globs
(`packages/*`, `apps/**`, and `!excluded` patterns). Outside a monorepo, only the
nearest `package.json` is used.

`pks <script>` runs the first exact match in the order above. If several
packages share a script name, run `pks` and filter to choose.

Try it on the [example monorepo](examples/monorepo).

## Development

```sh
cargo run -- -l
cargo test
cargo fmt && cargo clippy --all-targets -- -D warnings
```

### Releasing

1. Bump `version` in `Cargo.toml` and commit.
2. Tag and push:

   ```sh
   git tag v0.2.0
   git push origin v0.2.0
   ```

The release workflow builds binaries for macOS (arm64, x64), Linux (arm64, x64)
and Windows (x64), then publishes them with checksums as a GitHub release. The
tag must match the version in `Cargo.toml`.

## License

[MIT](LICENSE)
