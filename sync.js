const { S3Client, PutObjectCommand, HeadObjectCommand } = require("@aws-sdk/client-s3");
const mm = require("music-metadata");
const fs = require("fs");
const path = require("path");
require("dotenv").config();

// R2の設定 (S3互換APIを使用)
const endpoint = `https://${process.env.R2_ACCOUNT_ID}.r2.cloudflarestorage.com`;
console.log(`Connecting to R2 endpoint: ${endpoint}`);

const s3 = new S3Client({
  region: "auto",
  endpoint: endpoint,
  credentials: {
    accessKeyId: process.env.R2_ACCESS_KEY_ID,
    secretAccessKey: process.env.R2_SECRET_ACCESS_KEY,
  },
  // タイムアウト設定の強化
  requestHandler: {
    connectionTimeout: 60000,
    requestTimeout: 60000,
  }
});

const BUCKET = process.env.R2_BUCKET_NAME;
const MUSIC_DIR = path.join(require("os").homedir(), "Music"); // GNOME標準のミュージックフォルダ

async function sync() {
  if (!fs.existsSync(MUSIC_DIR)) {
    console.log(`Error: ${MUSIC_DIR} directory not found.`);
    return;
  }

  const files = getAllFiles(MUSIC_DIR);
  const index = [];

  console.log(`Found ${files.length} files. Starting sync...`);

  for (const file of files) {
    const ext = path.extname(file).toLowerCase();
    if (![".mp3", ".mp4", ".lrc"].includes(ext)) continue;

    const relativePath = path.relative(MUSIC_DIR, file).replace(/\\/g, "/");
    
    if (ext === ".mp3" || ext === ".mp4") {
      try {
        const metadata = await mm.parseFile(file);
        const track = {
          path: relativePath,
          title: metadata.common.title || path.basename(file, ext),
          artist: metadata.common.artist || "Unknown Artist",
          album: metadata.common.album || "Unknown Album",
          duration: metadata.format.duration || 0,
          track_number: metadata.common.track.no || null,
          lrc: null,
          video: ext === ".mp4" ? relativePath : null
        };

        const lrcPath = file.replace(ext, ".lrc");
        if (fs.existsSync(lrcPath)) {
          track.lrc = relativePath.replace(ext, ".lrc");
        }

        index.push(track);
        console.log(`Processing: ${track.title} - ${track.artist}`);

        await uploadToR2(file, relativePath);
        
        if (track.lrc) {
          await uploadToR2(lrcPath, track.lrc);
        }

      } catch (e) {
        console.error(`Error processing ${file}:`, e.message);
      }
    }
  }

  const indexContent = JSON.stringify(index, null, 2);
  fs.writeFileSync("music_index.json", indexContent);
  // インデックスファイルは常に更新する
  await uploadToR2("music_index.json", "music_index.json", "application/json", true);

  console.log("\nSync complete! music_index.json has been updated on R2.");
}

async function uploadToR2(localPath, r2Key, contentType = null, force = false) {
  const stats = fs.statSync(localPath);

  if (!force) {
    try {
      const headCommand = new HeadObjectCommand({
        Bucket: BUCKET,
        Key: r2Key,
      });
      const remoteData = await s3.send(headCommand);
      
      // サイズが同じならスキップ
      if (remoteData.ContentLength === stats.size) {
        console.log(`Skipped (Matches R2): ${r2Key}`);
        return;
      }
    } catch (err) {
      // 404 (NotFound) 以外のエラーは警告を出す
      if (err.name !== "NotFound" && err.$metadata?.httpStatusCode !== 404) {
        console.warn(`Check failed for ${r2Key}:`, err.message);
      }
    }
  }

  const fileStream = fs.createReadStream(localPath);
  const command = new PutObjectCommand({
    Bucket: BUCKET,
    Key: r2Key,
    Body: fileStream,
    ContentLength: stats.size,
    ContentType: contentType || getContentType(r2Key),
  });

  try {
    await s3.send(command);
    console.log(`Uploaded: ${r2Key} (${(stats.size / 1024 / 1024).toFixed(2)} MB)`);
  } catch (err) {
    console.error(`Upload error for ${r2Key}:`, err.message || err);
  }
}

function getAllFiles(dir, allFiles = []) {
  const files = fs.readdirSync(dir);
  files.forEach(file => {
    const name = path.join(dir, file);
    if (fs.statSync(name).isDirectory()) {
      getAllFiles(name, allFiles);
    } else {
      allFiles.push(name);
    }
  });
  return allFiles;
}

function getContentType(filename) {
  const ext = path.extname(filename).toLowerCase();
  if (ext === ".mp3") return "audio/mpeg";
  if (ext === ".mp4") return "video/mp4";
  if (ext === ".lrc") return "text/plain";
  return "application/octet-stream";
}

sync();
