export default {
  async fetch(request, env) {
    const url = new URL(request.url);
    const path = url.pathname;

    // --- 1. 曲一覧取得 (/tracks) ---
    // 巨大なJSONから必要なフィールド（最小限）だけを抜き出して送る
    if (path === "/tracks") {
      const indexFile = await env.MUSIC_BUCKET.get("music_index.json");
      if (!indexFile) return new Response("music_index.json not found", { status: 404 });

      try {
        const fullData = await indexFile.json();
        
        // 58MB -> 数百KBへ圧縮（重要！）
        const ultraLightData = fullData.map(track => ({
          path: track.path,
          title: track.title,
          artist: track.artist,
          album: track.album,
          duration: track.duration
        }));

        return new Response(JSON.stringify(ultraLightData), {
          headers: {
            "Content-Type": "application/json",
            "Access-Control-Allow-Origin": "*"
          }
        });
      } catch (e) {
        return new Response(`Error: ${e.message}`, { status: 500 });
      }
    }

    // --- 2. 歌詞取得 (/lyrics/:id) ---
    if (path.startsWith("/lyrics/")) {
      const id = path.replace("/lyrics/", "");
      const lrc = await env.MUSIC_BUCKET.get(`${id}.lrc`);
      if (!lrc) return new Response("No lyrics", { status: 404 });
      return new Response(lrc.body, { headers: { "Access-Control-Allow-Origin": "*" } });
    }

    // --- 3. 音楽ストリーミング (/stream/:id) ---
    if (path.startsWith("/stream/")) {
      const id = path.replace("/stream/", "");
      const mp3 = await env.MUSIC_BUCKET.get(`${id}.mp3`);
      if (!mp3) return new Response("Not found", { status: 404 });
      return new Response(mp3.body, {
        headers: {
          "Content-Type": "audio/mpeg",
          "Access-Control-Allow-Origin": "*"
        }
      });
    }

    return new Response("Not found", { status: 404 });
  }
};
