# Decision History

Current truth belongs in PROJECT.md and linked design documents. This file preserves consequential superseding decisions.

## 2026-10-08 — TASK-012 release matrix (3-platform)

- **Linux runner pinned to `ubuntu-22.04`, not `ubuntu-latest`.** Newer base silently raises the
  produced binaries' glibc floor (24.04 → 2.39) and breaks AppImage bundling: linuxdeploy's bundled
  strip (binutils 2.35) dies on `.relr.dyn` sections of glibc ≥ 2.36 system libs and FUSE2 was
  renamed to `libfuse2t64` (tauri-apps/tauri#14796). 22.04 keeps glibc floor at 2.35 = the documented
  "Ubuntu 22.04+/Debian 12+" requirement. Supersedes the brief's `ubuntu-latest` suggestion.
- **`prepare-release` job creates the GitHub Release up front.** Parallel platform jobs racing
  `action-gh-release`'s create path can hit `422 already_exists` or produce duplicate releases
  (softprops/action-gh-release#616/#705). With the release materialized in a seconds-long job,
  every publish step only ever takes the update path. Windows job needs no `needs:` — its publish
  step runs ~20min into the build, long after prepare-release finishes.
- **macOS ships two single-arch dmgs (aarch64 native + x86_64 cross) rather than universal2.**
  Each artifact is the vanilla `tauri build --target` path — lowest-risk option that can't be
  verified locally; arch failures are isolated (`fail-fast: false`); Intel Macs stay covered.
  universal2 remains a follow-up option once a real Mac can smoke-test it.
- **Linux bundles pinned to `--bundles deb,appimage`.** `targets:"all"` in tauri.conf.json would
  also attempt rpm on Linux, which requires `rpmbuild` we intentionally don't install.
- **`APPIMAGE_EXTRACT_AND_RUN=1` + `NO_STRIP=true` on the linux job.** CI runners can't reliably
  FUSE-mount linuxdeploy/appimagetool (themselves AppImages); NO_STRIP future-proofs a runner bump
  to 24.04. Both inert when unnecessary.
