let index = 0;

export async function handleBalancer(env, req) {
  const url = new URL(req.url);
  const path = url.pathname;

  const domain = "PLACEHOLDER_DOMAIN";
  const hosts = Array.from({ length: 30 }, (_, i) => `quadpro${i + 1}.${domain}`);

  // Endpoint: health check
  if (path === "/health") {
    return new Response("✅ Load balancer is up (stateless)", {
      headers: { "Content-Type": "text/plain" }
    });
  }

  // Pilih host round-robin (tidak persist)
  const selectedHost = hosts[index % hosts.length];
  index++;

  const targetUrl = new URL(req.url);
  targetUrl.hostname = selectedHost;

  // Jika backend hanya HTTP (bukan HTTPS), aktifkan ini:
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
