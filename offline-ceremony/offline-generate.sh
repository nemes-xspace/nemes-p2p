#!/bin/bash
# OFFLINE calistir. Internet KAPALI olmali. Bu script sifreyi EKRANA BASMAZ, sadece karta yazar.
set -euo pipefail
OUT="${1:-/media/KART1/nemes-kasa}"
mkdir -p "$OUT"
# 1) 1024 karakterlik sifre (/dev/urandom, yazdirilabilir alfabe)
LC_ALL=C tr -dc 'A-Za-z0-9!@#$%^&*()-_=+[]{}<>?/' < /dev/urandom | head -c 1024 > "$OUT/.sifre-1024.bin" || true
chmod 600 "$OUT/.sifre-1024.bin"
# 2) Master Ed25519 anahtari (offline, openssl)
openssl genpkey -algorithm ed25519 -out "$OUT/master-ed25519.pem"
chmod 600 "$OUT/master-ed25519.pem"
openssl pkey -in "$OUT/master-ed25519.pem" -pubout -out "$OUT/master-ed25519-pub.pem"
# 3) Cihaz parmak izi (kart seri + FS UUID) -> whitelist
lsblk -o NAME,MODEL,SERIAL,UUID,MOUNTPOINT > "$OUT/.cihaz-parmak-izi.txt" 2>/dev/null || script -qec "lsblk -o NAME,MODEL,SERIAL,UUID,MOUNTPOINT" /dev/null > "$OUT/.cihaz-parmak-izi.txt" 2>/dev/null || true
echo "OK: $OUT altina yazildi. Sifre dosyasi burada, hicbir yere yapistirma."
echo "PUBKEY (miner binary'lerine gomulecek TEK acik bilgi):"
cat "$OUT/master-ed25519-pub.pem"
