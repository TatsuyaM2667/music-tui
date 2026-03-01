export default {
  async fetch(request, env) {
    const url = new URL(request.url);
    const path = url.pathname;

    // --- 1. 曲一覧取得 (/tracks) ---
    // 巨大な画像データを除去して、アプリの起動を爆速にする
    if (path === "/tracks") {
      const indexFile = await env.MUSIC_BUCKET.get("music_index.json");
      if (!indexFile) return new Response("music_index.json not found", { status: 404 });

      try {
        // 58MBのJSONを読み込む
        const fullData = await indexFile.json();

        // 各曲のデータから 'cover' フィールド（巨大な画像）だけを削除する
        const lightData = fullData.map(track => {
          const { cover, ...rest } = track; // coverを除いた残りを取得
          return rest;
        });

        return new Response(JSON.stringify(lightData), {
          headers: {
            "Content-Type": "application/json",
            "Access-Control-Allow-Origin": "*"
          }
        });
      } catch (e) {
        return new Response(`Error processing JSON: ${e.message}`, { status: 500 });
      }
    }

    // --- 2. 歌詞取得 (/lyrics/:id) ---
    if (path.startsWith("/lyrics/")) {
      const id = path.replace("/lyrics/", "");
      const lrc = await env.MUSIC_BUCKET.get(`${id}.lrc`);
      if (!lrc) return new Response("lyrics not found", { status: 404 });

      return new Response(lrc.body, {
        headers: {
          "Content-Type": "text/plain",
    "Access-Control-Allow-Origin": "*"
        }
      });
    }

    // --- 3. 音楽ストリーミング (/stream/:id) ---
    if (path.startsWith("/stream/")) {
      const id = path.replace("/stream/", "");
      const mp3 = await env.MUSIC_BUCKET.get(`${id}.mp3`);
      if (!mp3) return new Response("audio not found", { status: 404 });

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
