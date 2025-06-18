#!/bin/bash

for i in {1..2}; do
  name="siren$i"
  domain="zxc$i.sumbangsih.dpdns.org"

  echo "🚀 Deploying $name → $domain"

  cp wrangler.toml.bak wrangler.temp.toml

  # Ganti name di bagian atas
  sed -i "s/^name = \".*\"/name = \"$name\"/" wrangler.temp.toml

  # Ganti baris routes yang sudah ada, atau tambahkan jika belum ada
  if grep -q "^routes = " wrangler.temp.toml; then
    sed -i "s|^routes = .*|routes = [\n  { pattern = \"$domain\", custom_domain = true }\n]|" wrangler.temp.toml
  else
    echo -e "\nroutes = [\n  { pattern = \"$domain\", custom_domain = true }\n]" >> wrangler.temp.toml
  fi

  # Deploy
  wrangler deploy --config wrangler.temp.toml

  echo "✅ Done $name → $domain"
done
