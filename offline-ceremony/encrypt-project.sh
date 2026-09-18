#!/bin/bash
# OFFLINE calistir. Kodu arsivleyip 1024-karakter sifreyle sifreler (AES-256 + PBKDF2).
set -euo pipefail
KART="${1:-/media/KART1/nemes-kasa}"
ARSIV_ADI="${2:-nemes-kaynak-arsiv}"
TMP="$(mktemp -d)"
trap 'rm -rf "$TMP"' EXIT
# 1) Temiz kaynak listesi: build ciktisi ve beyin verisi HARIC
tar --exclude='target' --exclude='node_modules' --exclude='dist' --exclude='*.log' --exclude='*.part' \
  -cf "$TMP/kaynak.tar" \
  /home/d3str0y1ng/nemes-p2p/nemes-core \
  /home/d3str0y1ng/nemes-p2p/nemes-p2p \
  /home/d3str0y1ng/nemes-p2p/nemes-cli \
  /home/d3str0y1ng/nemes-p2p/nemes-wallet \
  /home/d3str0y1ng/nemes-p2p/nemes-node \
  /home/d3str0y1ng/nemes-p2p/nemes-protocol \
  /home/d3str0y1ng/nemes-p2p/nemes-storage \
  /home/d3str0y1ng/nemes-p2p/nemes-consensus \
  /home/d3str0y1ng/nemes-p2p/nemes-embedding \
  /home/d3str0y1ng/nemes-p2p/Cargo.toml \
  /home/d3str0y1ng/nemes-p2p/offline-ceremony \
  /home/d3str0y1ng/NemesXSpace/00-PROJE-TANIMI.md \
  /home/d3str0y1ng/NemesXSpace/DURUM.md \
  /home/d3str0y1ng/NemesXSpace/README.md \
  /home/d3str0y1ng/NemesXSpace/miner \
  /home/d3str0y1ng/NemesXSpace/model 2>/dev/null || true
# 2) Manifest (butunluk)
(cd "$TMP" && sha256sum kaynak.tar > MANIFEST.sha256)
# 3) Sifrele: sifre dosyasi kartta, parola sorulmaz (batch)
openssl enc -e -aes-256-cbc -pbkdf2 -iter 2000000 -salt \
  -in "$TMP/kaynak.tar" -out "$KART/$ARSIV_ADI.tar.enc" \
  -pass "file:$KART/.sifre-1024.bin"
cp "$TMP/MANIFEST.sha256" "$KART/$ARSIV_ADI.MANIFEST.sha256"
# 4) Yedek karta birebir kopya (2. kart takiliyken ayni komut, hedef degisir)
echo "OK: $KART/$ARSIV_ADI.tar.enc yazildi."
ls -lh "$KART/$ARSIV_ADI".*
