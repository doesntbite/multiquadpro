export async function handleBalancer(env, req) {
  const url = new URL(req.url);
  const path = url.pathname;
  const kv = env.BALANCER_KV;
  const domain = env.DOMAIN;
  const hosts = Array.from({ length: 30 }, (_, i) => `quadpro${i + 1}.${domain}`);

  // Endpoint: reset statistik
  if (url.searchParams.get("reset") === "1") {
    await kv.delete("hostStats");
    await kv.delete("totalRequests");
    return new Response("✅ KV state reset");
  }

  // Endpoint: health check
  if (path === "/health") {
    const statsJson = await kv.get("hostStats");
    const stats = statsJson ? JSON.parse(statsJson) : {};
    const total = parseInt(await kv.get("totalRequests") || "0");

    return new Response(JSON.stringify({
      totalRequests: total,
      hostStats: stats,
    }, null, 2), {
      headers: { "Content-Type": "application/json" }
    });
  }

  // Load atau inisialisasi statistik
  const statsJson = await kv.get("hostStats");
  let stats = statsJson
    ? JSON.parse(statsJson)
    : Object.fromEntries(hosts.map(h => [h, 0]));

  // Pilih host dengan request paling sedikit
  const sorted = Object.entries(stats).sort((a, b) => a[1] - b[1]);
  const selectedHost = sorted[0][0];

  // Update statistik
  stats[selectedHost]++;
  const total = (parseInt(await kv.get("totalRequests") || "0")) + 1;

  await kv.put("hostStats", JSON.stringify(stats));
  await kv.put("totalRequests", total.toString());

  console.log(`🔄 Request #${total}: ${selectedHost}`);

  // Proxy request ke host
  const targetUrl = new URL(req.url);
  targetUrl.hostname = selectedHost;

  // Jika backend hanya HTTP (bukan HTTPS), uncomment ini:
  // targetUrl.protocol = "http:";

  const proxyReq = new Request(targetUrl.toString(), {
    method: req.method,
    headers: req.headers,
    body: req.method !== "GET" && req.method !== "HEAD" ? req.body : undefined,
    redirect: "manual"
  });

  const proxyResp = await fetch(proxyReq);

  return new Response(proxyResp.body, {
    status: proxyResp.status,
    statusText: proxyResp.statusText,
    headers: proxyResp.headers
  });
}
