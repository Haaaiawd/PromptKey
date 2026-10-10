#!/usr/bin/env bash
# verify_release_artifacts.sh — sanity-check release artifacts before they are
# attached to a GitHub Release (and again after download, per release checklist).
#
# Catches the realistic failure modes of a build→upload pipeline: zero-byte
# files, truncated uploads, HTML error pages saved under a binary name, and
# bundler output in the wrong format. It does NOT validate payload correctness.
#
# Usage:
#   verify_release_artifacts.sh [--strict] FILE [FILE ...]
#   verify_release_artifacts.sh --selftest
#
#   --strict    also require the executable bit on .AppImage. Used in CI right
#               after `tauri build`. Default is warn-only: files fetched back
#               over HTTP lose mode bits, so downloaded assets legitimately
#               arrive non-executable.
#
# Per-format checks (dispatched on lowercase extension):
#   .exe        'MZ' header + 'PE\0\0' signature at the e_lfanew offset (NSIS)
#   .msi        OLE2 compound-document magic D0 CF 11 E0 A1 B1 1A E1
#   .deb        ar magic '!<arch>\n' + members debian-binary / control.tar.* /
#               data.tar.* listed by `ar t`
#   .appimage   ELF magic + type-2 AppImage marker 'AI\x02' at offset 8
#   .dmg        'koly' UDIF trailer in the last 512 bytes
#   .tar.gz     gzip magic 1F 8B (optional updater .app.tar.gz)
#
# Size floors sit roughly an order of magnitude below observed real sizes —
# they exist to catch empty/truncated artifacts, not to grade content:
#   exe 1 MiB (2.0.5 NSIS exe ≈ 3.6 MB), msi 1 MiB (2.0.5 msi ≈ 4.9 MB),
#   deb 1 MiB (bundled rust binary ⇒ several MB after xz), AppImage 10 MiB
#   (bundles WebKitGTK ⇒ typically tens of MB), dmg 1 MiB, tar.gz 256 KiB.

set -uo pipefail

STRICT=0
SELFTEST=0
FILES=()
for arg in "$@"; do
  case "$arg" in
    --strict) STRICT=1 ;;
    --selftest) SELFTEST=1 ;;
    -h|--help) sed -n '2,33p' "$0"; exit 0 ;;
    *) FILES+=("$arg") ;;
  esac
done

FAILURES=0
err() { echo "    FAIL: $*" >&2; FAILURES=$((FAILURES + 1)); }

# hex_at FILE OFFSET COUNT → lowercase hex string of COUNT bytes at OFFSET
hex_at() {
  od -An -tx1 -j"$2" -N"$3" -- "$1" 2>/dev/null | tr -d ' \n'
}

# filesize FILE → byte count (GNU + BSD stat)
filesize() {
  stat -c %s -- "$1" 2>/dev/null || stat -f %z -- "$1"
}

# expect_magic FILE OFFSET EXPECTED_HEX LABEL
expect_magic() {
  local f=$1 off=$2 want=$3 label=$4
  local got
  got=$(hex_at "$f" "$off" "${#want}" )
  # ${#want} is hex chars; od -N takes bytes → half
  got=$(hex_at "$f" "$off" $(( ${#want} / 2 )))
  if [ "$got" != "$want" ]; then
    err "$f: bad $label magic (got '${got:-<eof>}', want '$want')"
    return 1
  fi
  return 0
}

# min_size FILE BYTES
min_size() {
  local f=$1 min=$2 sz
  sz=$(filesize "$f")
  if [ -z "$sz" ] || [ "$sz" -lt "$min" ]; then
    err "$f: size ${sz:-?}B below floor ${min}B (empty or truncated artifact)"
    return 1
  fi
  return 0
}

check_one() {
  local f=$1 ext base
  if [ ! -f "$f" ]; then err "$f: file not found (unmatched glob?)"; return; fi
  base=$(basename -- "$f")
  ext=$(echo "${base##*.}" | tr 'A-Z' 'a-z')
  echo "  checking $f ($(filesize "$f") bytes)"
  case "$base" in
    *.AppImage|*.appimage)
      expect_magic "$f" 0  "7f454c46" "ELF" || return
      expect_magic "$f" 8  "414902"   "type-2 AppImage marker (offset 8)" || return
      if [ -x "$f" ]; then
        :
      elif [ "$STRICT" = 1 ]; then
        err "$f: AppImage is not executable (bundler output must be +x)"
        return
      else
        echo "    warn: $f not executable (expected for HTTP-downloaded assets; users run chmod +x)"
      fi
      min_size "$f" 10485760 || return
      ;;
    *.exe)
      expect_magic "$f" 0 "4d5a" "MZ" || return
      # e_lfanew: little-endian uint32 at offset 0x3C points to the PE signature
      local eol
      eol=$(od -An -tu4 -j60 -N4 -- "$f" 2>/dev/null | tr -d ' ')
      if [ -z "$eol" ]; then err "$f: truncated before e_lfanew (DOS header)"; return; fi
      expect_magic "$f" "$eol" "50450000" "PE signature" || return
      min_size "$f" 1048576 || return
      ;;
    *.msi)
      expect_magic "$f" 0 "d0cf11e0a1b11ae1" "OLE2 compound" || return
      min_size "$f" 1048576 || return
      ;;
    *.deb)
      expect_magic "$f" 0 "213c617263683e0a" "ar archive (!<arch>)" || return
      local members
      members=$(ar t -- "$f" 2>/dev/null) || { err "$f: 'ar t' failed — not a readable archive"; return; }
      echo "$members" | grep -qx 'debian-binary'  || { err "$f: missing debian-binary member"; return; }
      echo "$members" | grep -qx 'control.tar.*'  || { err "$f: missing control.tar.* member"; return; }
      echo "$members" | grep -qx 'data.tar.*'     || { err "$f: missing data.tar.* member"; return; }
      min_size "$f" 1048576 || return
      ;;
    *.dmg)
      # UDIF trailer ('koly') sits at the start of the final 512-byte block
      local got
      got=$(tail -c 512 -- "$f" 2>/dev/null | od -An -tx1 -N4 | tr -d ' \n')
      if [ "$got" != "6b6f6c79" ]; then err "$f: no 'koly' trailer — not a UDIF dmg (got '${got:-<eof>}')"; return; fi
      min_size "$f" 1048576 || return
      ;;
    *.tar.gz|*.tgz)
      expect_magic "$f" 0 "1f8b" "gzip" || return
      min_size "$f" 262144 || return
      ;;
    *)
      err "$f: no verifier for extension '$ext' — add one, don't ship unchecked artifacts"
      return
      ;;
  esac
  echo "    OK ($ext)"
}

selftest() {
  # Fabricates minimal-format fixtures and asserts the verifier accepts the
  # good ones and rejects the broken ones. Needs GNU tools (truncate, dd, ar);
  # intended for developer machines and CI, not for the runners' hot path.
  local d; d=$(mktemp -d)
  trap 'rm -rf "$d"' RETURN
  local sz=2097152 big=12582912

  # good fixtures -------------------------------------------------------
  printf 'MZ' > "$d/ok_x64-setup.exe"
  printf '\x80\x00\x00\x00' | dd of="$d/ok_x64-setup.exe" bs=1 seek=60 conv=notrunc status=none
  printf 'PE\x00\x00'      | dd of="$d/ok_x64-setup.exe" bs=1 seek=128 conv=notrunc status=none
  truncate -s "$sz" "$d/ok_x64-setup.exe"

  printf '\xd0\xcf\x11\xe0\xa1\xb1\x1a\xe1' > "$d/ok_x64_en-US.msi"
  truncate -s "$sz" "$d/ok_x64_en-US.msi"

  # real ar archive with the three required member names
  ( cd "$d"
    echo '2.0' > debian-binary
    : > control.tar.xz
    truncate -s "$sz" data.tar.xz
    ar rcs good_amd64.deb debian-binary control.tar.xz data.tar.xz
    rm -f debian-binary control.tar.xz data.tar.xz )

  printf '\x7f\x45\x4c\x46\x02\x01\x01\x00' > "$d/ok_amd64.AppImage"
  printf '\x41\x49\x02' | dd of="$d/ok_amd64.AppImage" bs=1 seek=8 conv=notrunc status=none
  truncate -s "$big" "$d/ok_amd64.AppImage"
  chmod +x "$d/ok_amd64.AppImage"

  truncate -s "$sz" "$d/ok_aarch64.dmg"
  printf 'koly' | dd of="$d/ok_aarch64.dmg" bs=1 seek=$((sz - 512)) conv=notrunc status=none

  printf '\x1f\x8b\x08\x00' > "$d/ok.app.tar.gz"
  truncate -s 524288 "$d/ok.app.tar.gz"

  # bad fixtures --------------------------------------------------------
  printf 'MZ' > "$d/bad_no_pe.exe"                                   # MZ but no PE sig
  truncate -s "$sz" "$d/bad_no_pe.exe"
  printf '<html><body>404</body></html>' > "$d/bad_html.exe"          # error page as .exe
  : > "$d/bad_empty.deb"                                              # zero bytes
  printf '\x7f\x45\x4c\x46\x02\x01\x01\x00' > "$d/bad_magic.AppImage" # ELF but no AI\x02
  truncate -s "$big" "$d/bad_magic.AppImage"; chmod +x "$d/bad_magic.AppImage"
  cp "$d/ok_amd64.AppImage" "$d/bad_noexec.AppImage"; chmod -x "$d/bad_noexec.AppImage"
  truncate -s "$sz" "$d/bad_trailer.dmg"                              # no koly trailer
  printf '\x21\x3c\x61\x72\x63\x68\x3e\x0a' > "$d/bad_members.deb"    # ar magic, no members

  local pass=0 fail=0 t
  for f in "$d/ok_x64-setup.exe" "$d/ok_x64_en-US.msi" "$d/good_amd64.deb" \
           "$d/ok_amd64.AppImage" "$d/ok_aarch64.dmg" "$d/ok.app.tar.gz"; do
    echo "selftest expect-PASS $f"
    if FAILURES=0; check_one "$f" && [ "$FAILURES" = 0 ]; then pass=$((pass+1)); else fail=$((fail+1)); echo "    !! expected pass"; fi
  done
  for f in "$d/bad_no_pe.exe" "$d/bad_html.exe" "$d/bad_empty.deb" \
           "$d/bad_magic.AppImage" "$d/bad_trailer.dmg" "$d/bad_members.deb"; do
    echo "selftest expect-FAIL $f"
    if FAILURES=0; check_one "$f" && [ "$FAILURES" = 0 ]; then fail=$((fail+1)); echo "    !! expected FAIL but passed"; else pass=$((pass+1)); fi
  done
  # strict mode must additionally reject the non-executable AppImage
  STRICT=1; echo "selftest expect-FAIL(--strict) $d/bad_noexec.AppImage"
  if FAILURES=0; check_one "$d/bad_noexec.AppImage" && [ "$FAILURES" = 0 ]; then
    fail=$((fail+1)); echo "    !! expected FAIL under --strict but passed"
  else pass=$((pass+1)); fi
  STRICT=0

  echo "selftest: $pass passed, $fail failed"
  [ "$fail" = 0 ]
}

if [ "$SELFTEST" = 1 ]; then
  selftest
  exit $?
fi

if [ "${#FILES[@]}" = 0 ]; then
  echo "usage: $0 [--strict] FILE [FILE ...]   |   $0 --selftest" >&2
  exit 2
fi

echo "verify_release_artifacts: ${#FILES[@]} file(s), strict=$STRICT"
for f in "${FILES[@]}"; do
  check_one "$f"
done

if [ "$FAILURES" -gt 0 ]; then
  echo "verify_release_artifacts: $FAILURES check(s) FAILED" >&2
  exit 1
fi
echo "verify_release_artifacts: all artifacts OK"
