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
        title: track.title,
        artist: track.artist,
        album: track.album,
        duration: track.duration
      })).join('\n');
      return new Response(chunks, { headers: { "Content-Type": "application/x-ndjson", "Access-Control-Allow-Origin": "*" } });
    }

    // --- 2. ストリーミング & 歌詞 ---
    let type = null;
    let rawKey = null;

    if (path.startsWith("/stream/")) {
      type = "audio/mpeg";
      rawKey = path.replace("/stream/", "");
    } else if (path.startsWith("/lyrics/")) {
      type = "text/plain";
      rawKey = path.replace("/lyrics/", "");
    }

    if (rawKey) {
      // R2のキーとして考えられるパターンをすべて試す (スラッシュの有無など)
      const keysToTry = [
        rawKey,                          // そのまま
        rawKey.startsWith("/") ? rawKey.slice(1) : rawKey, // 先頭のスラッシュ削除
        rawKey.startsWith("/") ? rawKey : "/" + rawKey     // 先頭のスラッシュ追加
      ];

      for (const key of keysToTry) {
        const file = await env.MUSIC_BUCKET.get(key);
        if (file) {
          return new Response(file.body, {
            headers: { "Content-Type": type, "Access-Control-Allow-Origin": "*" }
          });
        }
      }

      // すべて失敗した場合
      return new Response(`R2 Key Not Found. Tried: ${JSON.stringify(keysToTry)}`, { status: 404 });
    }

    return new Response("Not found", { status: 404 });
  }
};
