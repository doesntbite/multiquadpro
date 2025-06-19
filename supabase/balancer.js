export async function handleBalancer(env, req) {
  const url = new URL(req.url);
  const path = url.pathname;
  const supabaseUrl = env.SUPABASE_URL;
  const supabaseKey = env.SUPABASE_ANON_KEY;

  const domain = "PLACEHOLDER_DOMAIN";
  const hosts = Array.from({ length: 30 }, (_, i) => `quadpro${i + 1}.${domain}`);

  // Health check
  if (path === "/health") {
    return new Response("✅ Load balancer is up (Supabase)", {
      headers: { "Content-Type": "text/plain" }
    });
  }

  // Stats dashboard
  if (path === "/stats") {
    const statsResp = await fetch(`${supabaseUrl}/rest/v1/host_stats?select=host,count`, {
      headers: {
        "apikey": supabaseKey,
        "Authorization": `Bearer ${supabaseKey}`
      }
    });

    if (!statsResp.ok) {
      return new Response("Failed to fetch stats", { status: 500 });
    }

    const stats = await statsResp.json();

    // Aggregate per domain
    const domainCounts = {};
    for (const { host, count } of stats) {
      const domainPart = host.split('.').slice(1).join('.');
      domainCounts[domainPart] = (domainCounts[domainPart] || 0) + count;
    }

    // Sort hosts by count descending
    stats.sort((a, b) => b.count - a.count);

    // Build simple HTML dashboard
    const html = `
      <html>
        <head>
          <title>Load Balancer Stats Dashboard</title>
          <style>
            body { font-family: Arial, sans-serif; padding: 20px; }
            table { border-collapse: collapse; width: 100%; margin-bottom: 20px; }
            th, td { border: 1px solid #ddd; padding: 8px; text-align: left; }
            th { background-color: #f4f4f4; }
            h2 { margin-top: 40px; }
          </style>
        </head>
        <body>
          <h1>Load Balancer Stats Dashboard</h1>

          <h2>Top Hosts</h2>
          <table>
            <thead><tr><th>Host</th><th>Request Count</th></tr></thead>
            <tbody>
              ${stats.map(s => `<tr><td>${s.host}</td><td>${s.count}</td></tr>`).join('')}
            </tbody>
          </table>

          <h2>Request Count per Domain</h2>
          <table>
            <thead><tr><th>Domain</th><th>Request Count</th></tr></thead>
            <tbody>
              ${Object.entries(domainCounts).map(([domain, count]) => `<tr><td>${domain}</td><td>${count}</td></tr>`).join('')}
            </tbody>
          </table>
        </body>
      </html>
    `;

    return new Response(html, { headers: { "Content-Type": "text/html" } });
  }

  // Round-robin load balancing
  // (Example: simple round robin, stateless)
  let index = 0;
  const selectedHost = hosts[index % hosts.length];
  index++;

  // Proxy request
  const targetUrl = new URL(req.url);
  targetUrl.hostname = selectedHost;

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
