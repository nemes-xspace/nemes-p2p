#!/bin/bash
# Karta bagli kilit acma + dogrulama. Sifre RAM'de (/dev/shm) tutulur, diske yazilmaz.
set -euo pipefail
KART="${1:-/media/KART1/nemes-kasa}"
ARSIV_ADI="${2:-nemes-kaynak-arsiv}"
BEKLENEN_SERILER="${3:-}"  # ornek: "ABC123,XYZ789" -> iki kartin serisi de kabul edilir
# 1) Cihaz kontrolu (klon karta karsi ilk filtre)
GERCEK_SERILER="$(lsblk -o SERIAL -nr 2>/dev/null | tr '\n' ',' || true)"
if [ -n "$BEKLENEN_SERILER" ]; then
  OK=0
  IFS=',' read -ra LST <<< "$BEKLENEN_SERILER"
  for S in "${LST[@]}"; do
    case "$GERCEK_SERILER" in *"$S"*) OK=1;; esac
  done
  if [ "$OK" != "1" ]; then echo "RED: bu cihaz whitelist'te yok"; exit 1; fi
fi
# 2) Sifre dosyasi burada mi?
[ -f "$KART/.sifre-1024.bin" ] || { echo "RED: sifre dosyasi yok"; exit 1; }
# 3) Coz ve manifest dogrula (RAM uzerinden)
TMP="$(mktemp -d /dev/shm/nemes-XXXX)"
trap 'rm -rf "$TMP"' EXIT
openssl enc -d -aes-256-cbc -pbkdf2 -iter 2000000 \
  -in "$KART/$ARSIV_ADI.tar.enc" -out "$TMP/kaynak.tar" \
  -pass "file:$KART/.sifre-1024.bin"
(cd "$TMP" && sha256sum -c "$KART/$ARSIV_ADI.MANIFEST.sha256")
echo "OK: arsiv acildi ve manifest dogrulandi: $TMP/kaynak.tar"
echo "Isin bitince terminali kapat, /dev/shm otomatik silinir."
