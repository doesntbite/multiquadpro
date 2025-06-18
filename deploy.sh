#!/bin/bash

# List nama Worker yang ingin dideploy
workers=("siren" "siren2" "siren3" "siren4" "siren5")

# Backup wrangler.toml agar tidak rusak saat edit
cp wrangler.toml wrangler.toml.bak

for name in "${workers[@]}"; do
  echo "Deploying $name..."

  # Ganti baris name = "...", sesuaikan dengan nama Worker
  sed -i.bak "s/^name = \".*\"/name = \"$name\"/" wrangler.toml

  # Jalankan deploy
  wrangler publish
done

# Kembalikan wrangler.toml ke versi awal
mv wrangler.toml.bak wrangler.toml
rm -f wrangler.toml.bak.bak
