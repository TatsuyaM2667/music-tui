export default {
  async fetch(request, env) {
    const url = new URL(request.url);
    const path = url.pathname;

    if (path === "/tracks") {
      const indexFile = await env.MUSIC_BUCKET.get("music_index.json");
      if (!indexFile) return new Response("Not found", { status: 404 });

      const fullData = await indexFile.json();
      
      // 1曲1行の NDJSON 形式に変換
      // これにより、クライアント側で届いた順に「再生可能」として表示できる
      const chunks = fullData.map(track => JSON.stringify({
        path: track.path,
        title: track.title,
        artist: track.artist,
        album: track.album,
        duration: track.duration
      })).join('\n');

      return new Response(chunks, {
        headers: {
          "Content-Type": "application/x-ndjson",
          "Access-Control-Allow-Origin": "*"
        }
      });
    }

    if (path.startsWith("/lyrics/")) {
      const id = path.replace("/lyrics/", "");
      const lrc = await env.MUSIC_BUCKET.get(`${id}.lrc`);
      if (!lrc) return new Response("No lyrics", { status: 404 });
      return new Response(lrc.body, { headers: { "Access-Control-Allow-Origin": "*" } });
    }

    if (path.startsWith("/stream/")) {
      const id = path.replace("/stream/", "");
      const mp3 = await env.MUSIC_BUCKET.get(`${id}.mp3`);
      if (!mp3) return new Response("Not found", { status: 404 });
      return new Response(mp3.body, {
        headers: { "Content-Type": "audio/mpeg", "Access-Control-Allow-Origin": "*" }
      });
    }

    return new Response("Not found", { status: 404 });
  }
};
