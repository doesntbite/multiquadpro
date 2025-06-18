for i in {1..2}; do
  name="siren$i"
  domain="zxc$i.sumbangsih.dpdns.org"

  echo "🚀 Deploying $name → $domain"

  cp wrangler.toml.bak wrangler.temp.toml

  # Ganti nama worker
  sed -i "s/^name = \".*\"/name = \"$name\"/" wrangler.temp.toml

  # Tambahkan custom domain
  echo -e "\nroutes = [\n  { pattern = \"$domain\", custom_domain = true }\n]" >> wrangler.temp.toml

  # Deploy
  wrangler deploy --config wrangler.temp.toml

  echo "✅ Done $name → $domain"
done
