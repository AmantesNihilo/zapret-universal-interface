const FIXED_BYTES = new TextEncoder().encode("ZUI probe payload v1\n".repeat(2048));

const noStoreHeaders = {
  "Access-Control-Allow-Origin": "*",
  "Cache-Control": "no-store, max-age=0",
  "X-Content-Type-Options": "nosniff"
};

function json(value, status = 200) {
  return new Response(JSON.stringify(value), {
    status,
    headers: { ...noStoreHeaders, "Content-Type": "application/json; charset=utf-8" }
  });
}

function byteRange(request) {
  const range = request.headers.get("Range");
  if (!range) {
    return new Response(FIXED_BYTES, {
      headers: { ...noStoreHeaders, "Content-Type": "application/octet-stream", "Accept-Ranges": "bytes" }
    });
  }
  const match = /^bytes=(\d+)-(\d*)$/.exec(range);
  if (!match) return new Response(null, { status: 416, headers: noStoreHeaders });
  const start = Number(match[1]);
  const requestedEnd = match[2] ? Number(match[2]) : FIXED_BYTES.length - 1;
  const end = Math.min(requestedEnd, FIXED_BYTES.length - 1);
  if (!Number.isSafeInteger(start) || !Number.isSafeInteger(end) || start > end || start >= FIXED_BYTES.length) {
    return new Response(null, { status: 416, headers: noStoreHeaders });
  }
  return new Response(FIXED_BYTES.slice(start, end + 1), {
    status: 206,
    headers: {
      ...noStoreHeaders,
      "Content-Type": "application/octet-stream",
      "Accept-Ranges": "bytes",
      "Content-Range": `bytes ${start}-${end}/${FIXED_BYTES.length}`
    }
  });
}

function websocket(request) {
  if (request.headers.get("Upgrade")?.toLowerCase() !== "websocket") {
    return json({ ok: false, error: "websocket upgrade required" }, 426);
  }
  const pair = new WebSocketPair();
  const [client, server] = Object.values(pair);
  server.accept();
  server.addEventListener("message", (event) => {
    const value = typeof event.data === "string" ? event.data.slice(0, 4096) : event.data;
    server.send(value);
  });
  return new Response(null, { status: 101, webSocket: client });
}

export default {
  async fetch(request) {
    const url = new URL(request.url);
    if (request.method !== "GET") return json({ ok: false, error: "method not allowed" }, 405);
    if (url.pathname === "/health") return json({ ok: true, service: "zui-network-probe", version: 1 });
    if (url.pathname === "/json") return json({ ok: true, marker: "zui-probe-v1" });
    if (url.pathname === "/redirect") return Response.redirect(`${url.origin}/json`, 302);
    if (url.pathname === "/bytes") return byteRange(request);
    if (url.pathname === "/ws") return websocket(request);
    if (url.pathname === "/diagnostic") {
      return json({
        ok: true,
        marker: "zui-probe-v1",
        protocol: request.cf?.httpProtocol ?? "unknown",
        tlsVersion: request.cf?.tlsVersion ?? "unknown"
      });
    }
    return json({ ok: false, error: "not found" }, 404);
  }
};
