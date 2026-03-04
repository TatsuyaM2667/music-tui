const { S3Client, PutObjectCommand, HeadObjectCommand, GetObjectCommand, ListObjectsV2Command } = require("@aws-sdk/client-s3");
const mm = require("music-metadata");
const fs = require("fs");
const path = require("path");
const dns = require("dns");
require("dotenv").config();

// Node.js v17+ の IPv6 優先問題を回避
if (dns.setDefaultResultOrder) {
  dns.setDefaultResultOrder('ipv4first');
}

const endpoint = `https://${process.env.R2_ACCOUNT_ID}.r2.cloudflarestorage.com`;
const BUCKET = process.env.R2_BUCKET_NAME;
const MUSIC_DIR = path.join(require("os").homedir(), "Music");

const s3 = new S3Client({
  region: "auto",
  endpoint: endpoint,
  credentials: {
    accessKeyId: process.env.R2_ACCESS_KEY_ID,
    secretAccessKey: process.env.R2_SECRET_ACCESS_KEY,
  },
  requestHandler: {
    connectionTimeout: 60000,
    requestTimeout: 60000,
  }
});

async function sync() {
  if (!fs.existsSync(MUSIC_DIR)) {
    console.log(`Error: ${MUSIC_DIR} directory not found.`);
    return;
  }

  console.log(`Connecting to R2 endpoint: ${endpoint}`);
  console.log("Fetching existing files from R2...");
  const existingR2Keys = await listAllR2Objects();

  let indexMap = new Map();
  try {
    const getCmd = new GetObjectCommand({ Bucket: BUCKET, Key: "music_index.json" });
    const res = await s3.send(getCmd);
    const body = await res.Body.transformToString();
    const oldIndex = JSON.parse(body);
    oldIndex.forEach(t => indexMap.set(t.path, t));
    console.log(`Loaded ${indexMap.size} existing tracks from R2 index.`);
  } catch (e) {
    console.log("No existing index found or failed to load. Starting fresh.");
  }

  const localFiles = getAllFiles(MUSIC_DIR);
  console.log(`Found ${localFiles.length} local files. Syncing...`);

  for (const file of localFiles) {
    const ext = path.extname(file).toLowerCase();
    if (![".mp3", ".mp4", ".lrc"].includes(ext)) continue;

    const relativePath = path.relative(MUSIC_DIR, file).replace(/\\/g, "/");
    
    if (ext === ".mp3" || ext === ".mp4") {
      try {
        await uploadToR2(file, relativePath);

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
          await uploadToR2(lrcPath, track.lrc);
        }

        indexMap.set(relativePath, track);
        console.log(`Processed: ${track.title}`);
      } catch (e) {
        console.error(`Error processing ${file}:`, e.message);
      }
    }
  }

  const finalIndex = [];
  for (const [path, track] of indexMap) {
    if (existingR2Keys.has(path)) {
      finalIndex.push(track);
    } else {
      console.log(`Removing from index (Not on R2): ${path}`);
    }
  }

  const indexContent = JSON.stringify(finalIndex, null, 2);
  fs.writeFileSync("music_index.json", indexContent);
  await uploadToR2("music_index.json", "music_index.json", "application/json", true);

  console.log(`\nSync complete! Index now contains ${finalIndex.length} tracks.`);
}

async function listAllR2Objects() {
  const keys = new Set();
  let continuationToken = null;
  do {
    const cmd = new ListObjectsV2Command({
      Bucket: BUCKET,
      ContinuationToken: continuationToken
    });
    const res = await s3.send(cmd);
    if (res.Contents) {
      res.Contents.forEach(obj => keys.add(obj.Key));
    }
    continuationToken = res.NextContinuationToken;
  } while (continuationToken);
  return keys;
}

async function uploadToR2(localPath, r2Key, contentType = null, force = false) {
  const stats = fs.statSync(localPath);

  if (!force) {
    try {
      const headCommand = new HeadObjectCommand({ Bucket: BUCKET, Key: r2Key });
      const remoteData = await s3.send(headCommand);
      if (remoteData.ContentLength === stats.size) {
        return;
      }
    } catch (err) {
      // NotFound
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
    console.error(`Upload error for ${r2Key}:`, err.message);
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

sync().catch(err => {
  console.error("\n--- Sync Failed ---");
  console.error(err);
  process.exit(1);
});
