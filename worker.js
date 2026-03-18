export default {
  async fetch(request, env) {
    const url = new URL(request.url);
    const path = decodeURIComponent(url.pathname); // 確実にデコードされたパスを取得

    // --- 1. 曲一覧 ---
    if (path === "/tracks") {
      const indexFile = await env.MUSIC_BUCKET.get("music_index.json");
      if (!indexFile) return new Response("Not found", { status: 404 });
      const fullData = await indexFile.json();
      const chunks = fullData.map(track => JSON.stringify({
        path: track.path,
        lrc: track.lrc,
        video: track.video,
        title: track.title,
        artist: track.artist,
        album: track.album,
        duration: track.duration,
        track_number: track.track_number || null
      })).join('\n');
      return new Response(chunks, { headers: { "Content-Type": "application/x-ndjson", "Access-Control-Allow-Origin": "*" } });
    }

    // --- 1.1 曲順変更 ---
    if (path === "/reorder" && request.method === "POST") {
      try {
        const tracks = await request.json();
        if (!Array.isArray(tracks)) {
          return new Response("Invalid data format", { status: 400 });
        }
        await env.MUSIC_BUCKET.put("music_index.json", JSON.stringify(tracks));
        return new Response("OK", { headers: { "Access-Control-Allow-Origin": "*" } });
      } catch (e) {
        return new Response(e.message, { status: 500 });
      }
    }

    // --- 2. ストリーミング & 歌詞 ---
    let type = "application/octet-stream";
    let rawKey = null;

    if (path.startsWith("/stream/")) {
      rawKey = path.replace("/stream/", "");
      if (rawKey.toLowerCase().endsWith(".mp4")) {
        type = "video/mp4";
      } else if (rawKey.toLowerCase().endsWith(".mp3")) {
        type = "audio/mpeg";
      }
    } else if (path.startsWith("/lyrics/")) {
      type = "text/plain";
      rawKey = path.replace("/lyrics/", "");
    }

    if (path.startsWith("/stream/")) {
      const rawPath = url.pathname.replace("/stream/", ""); // エンコードされたままのパス
      const decodedPath = decodeURIComponent(rawPath);    // デコードされたパス
      
      const keysToTry = new Set([
        decodedPath,
        decodedPath.startsWith("/") ? decodedPath.slice(1) : decodedPath,
        decodedPath.startsWith("/") ? decodedPath : "/" + decodedPath,
        rawPath,
        rawPath.startsWith("/") ? rawPath.slice(1) : rawPath
      ]);

      for (const key of Array.from(keysToTry)) {
        const file = request.method === "HEAD"
          ? await env.MUSIC_BUCKET.head(key)
          : await env.MUSIC_BUCKET.get(key, {
              range: request.headers.get("range"),
              onlyIf: request.headers,
            });

        if (file) {
          const headers = new Headers();
          file.writeHttpMetadata(headers);
          headers.set("Access-Control-Allow-Origin", "*");
          headers.set("Content-Type", type);
          headers.set("Accept-Ranges", "bytes");

          const status = file.body 
            ? (request.headers.get("range") ? 206 : 200) 
            : (request.method === "HEAD" ? 200 : 304);

          return new Response(file.body, { headers, status });
        }
      }

      return new Response(`R2 Key Not Found. Path: ${decodedPath}, Tried: ${JSON.stringify(Array.from(keysToTry))}`, { status: 404 });
    }

    return new Response("Not found", { status: 404 });
  }
};
