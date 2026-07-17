import os
import json
import base64
import tempfile
import re
import unicodedata
from typing import List, Dict, Optional, Set
from urllib.parse import quote

import boto3
from mutagen.mp3 import MP3
from mutagen.id3 import ID3, ID3NoHeaderError

# === Cloudflare R2 Config (same as upload_r2.py) ===
ACCESS_KEY = os.getenv("R2_ACCESS_KEY", "80d3a67721604a9f48b05c23bcdc17f0")
SECRET_KEY = os.getenv("R2_SECRET_KEY", "0917a027a904e8906b58357e0b120bb1f03f5eed770edb3508e85557eb6773ec")
ENDPOINT = "https://7c08b6d98b0c6905fd40b348609b499b.r2.cloudflarestorage.com"
BUCKET = "music-server"

# === Output Config ===
OUTPUT_JSON = "music_index.json"
R2_PUBLIC_URL = ""  # Set to your R2 public URL if you have one (e.g., https://music.example.com)
                     # Leave empty to generate relative paths like "/key/path"

# === Processing Config ===
INCLUDE_COVER = True
MAX_COVER_BYTES = 512 * 1024  # 512KB limit for embedded cover art
PROGRESS_EVERY = 50
# ID3v2 tags are at the start of MP3 files. 1MB is enough for tags + embedded cover art.
PARTIAL_DOWNLOAD_BYTES = 1 * 1024 * 1024  # 1MB

IMAGE_EXTENSIONS = {
    ".jpg", ".jpeg", ".png", ".webp", ".avif", ".gif", ".bmp", ".tif", ".tiff", ".svg",
}
VIDEO_EXTENSIONS = {
    ".mp4", ".m4v", ".mov", ".webm", ".mkv", ".avi", ".wmv", ".flv", ".mpeg", ".mpg", ".3gp",
}
LYRIC_EXTENSIONS = {".lrc"}

# === Initialize S3 Client ===
s3 = boto3.client(
    "s3",
    endpoint_url=ENDPOINT,
    aws_access_key_id=ACCESS_KEY,
    aws_secret_access_key=SECRET_KEY,
)


def list_r2_keys() -> Dict[str, dict]:
    """
    List all objects in the R2 bucket.
    Returns a dict: { key: { 'Size': int, 'LastModified': datetime } }
    """
    objects = {}
    paginator = s3.get_paginator("list_objects_v2")
    print("[INFO] Fetching R2 bucket contents...")
    for page in paginator.paginate(Bucket=BUCKET):
        for obj in page.get("Contents", []):
            objects[obj["Key"]] = {
                "Size": obj["Size"],
                "LastModified": obj["LastModified"],
            }
    print(f"[INFO] Found {len(objects)} objects in R2.")
    return objects


def download_partial_from_r2(key: str, dest_path: str, max_bytes: int = PARTIAL_DOWNLOAD_BYTES) -> bool:
    """
    Download only the first max_bytes of a file from R2.
    ID3v2 tags are at the beginning of MP3 files, so we only need the header portion.
    """
    try:
        response = s3.get_object(
            Bucket=BUCKET,
            Key=key,
            Range=f"bytes=0-{max_bytes - 1}",
        )
        with open(dest_path, "wb") as f:
            f.write(response["Body"].read())
        return True
    except Exception as e:
        print(f"[WARN] Failed to download {key}: {e}")
        return False


def make_web_path(key: str, r2_url: str = "") -> str:
    """Generate a web-accessible path for the given R2 key."""
    if r2_url:
        return f"{r2_url}/{key}"
    else:
        return "/" + key


def split_artist_names(value: Optional[str]) -> List[str]:
    if not value:
        return []
    return [
        part.strip()
        for part in re.split(r"[,、/／&＆]|\s+(?:feat\.?|ft\.?|x|×)\s+", str(value), flags=re.IGNORECASE)
        if part.strip()
    ]


def normalize_name(value: str) -> str:
    return unicodedata.normalize("NFC", value).casefold().strip()


def find_related_key(all_keys: Set[str], base_key_no_ext: str, extensions: List[str]) -> Optional[str]:
    """
    Find a related file in R2 by checking possible extensions.
    base_key_no_ext: the key without extension (e.g. "Artist/Song")
    extensions: list of extensions to try (e.g. [".lrc"])
    """
    for ext in extensions:
        candidate = base_key_no_ext + ext
        if candidate in all_keys:
            return candidate
    return None


def find_video_key(all_keys: Set[str], base_key_no_ext: str) -> Optional[str]:
    """Find a matching video file key in R2."""
    video_extensions = [".mp4", ".MP4", ".m4v", ".M4V", ".mov", ".MOV"]
    return find_related_key(all_keys, base_key_no_ext, video_extensions)


def find_cover_key(all_keys: Set[str], dir_prefix: str, mp3_stem: str) -> Optional[str]:
    """Find an external cover image in the same R2 'directory'."""
    cover_names = ["cover", "folder", "front", "album", mp3_stem]
    cover_extensions = [".jpg", ".jpeg", ".png", ".webp"]

    for name in cover_names:
        for ext in cover_extensions:
            if dir_prefix:
                candidate = f"{dir_prefix}/{name}{ext}"
            else:
                candidate = f"{name}{ext}"
            if candidate in all_keys:
                return candidate
    return None


def find_artist_image_key(all_keys: Set[str], artist_name: str) -> Optional[str]:
    """
    Find an artist image in R2.
    Checks root level and common subdirectories (artists/, Artists/, etc.)
    """
    if not artist_name or artist_name == "Unknown":
        return None

    clean_name = artist_name.strip()
    normalized_clean_name = normalize_name(clean_name)
    image_extensions = [
        ".avif", ".webp", ".png", ".jpg", ".jpeg", ".gif", ".bmp",
        ".AVIF", ".WEBP", ".PNG", ".JPG", ".JPEG", ".GIF", ".BMP",
    ]

    # Search prefixes: root, and common artist image directories
    search_prefixes = ["", "artists/", "Artists/", "artist_images/", "images/"]

    for prefix in search_prefixes:
        for ext in image_extensions:
            candidate = f"{prefix}{clean_name}{ext}"
            if candidate in all_keys:
                return candidate

    # Case-insensitive fallback: scan all keys
    lower_name = normalized_clean_name
    for key in all_keys:
        # Extract the filename part (after last /)
        filename = key.rsplit("/", 1)[-1] if "/" in key else key
        stem, ext = os.path.splitext(filename)
        if ext.lower() in [e.lower() for e in image_extensions]:
            if normalize_name(stem) == lower_name:
                return key

    return None


def split_artist_names(value: Optional[str]) -> List[str]:
    if not value:
        return []
    return [
        part.strip()
        for part in re.split(r"\s*(?:,|、|&|/|\||feat\.?|ft\.?|with| x |×)\s*", str(value), flags=re.IGNORECASE)
        if part.strip()
    ]


def normalize_key(value: str) -> str:
    return unicodedata.normalize("NFC", value).casefold()


def build_normalized_key_map(all_keys: Set[str]) -> Dict[str, str]:
    """Map normalized R2 keys to their original spelling."""
    return {normalize_key(key): key for key in all_keys}


def has_extension(key: str, extensions: Set[str]) -> bool:
    return os.path.splitext(key)[1].casefold() in extensions


def find_related_key(all_keys: Set[str], base_key_no_ext: str, extensions: Set[str], normalized_keys: Optional[Dict[str, str]] = None) -> Optional[str]:
    key_map = normalized_keys or build_normalized_key_map(all_keys)
    for ext in extensions:
        candidate = key_map.get(normalize_key(base_key_no_ext + ext))
        if candidate:
            return candidate
    return None


def find_video_key(all_keys: Set[str], base_key_no_ext: str, normalized_keys: Optional[Dict[str, str]] = None) -> Optional[str]:
    return find_related_key(all_keys, base_key_no_ext, VIDEO_EXTENSIONS, normalized_keys)


def find_cover_key(all_keys: Set[str], dir_prefix: str, mp3_stem: str, normalized_keys: Optional[Dict[str, str]] = None) -> Optional[str]:
    cover_names = ["cover", "folder", "front", "album", mp3_stem]
    key_map = normalized_keys or build_normalized_key_map(all_keys)

    for name in cover_names:
        for ext in IMAGE_EXTENSIONS:
            candidate_path = f"{dir_prefix}/{name}{ext}" if dir_prefix else f"{name}{ext}"
            candidate = key_map.get(normalize_key(candidate_path))
            if candidate:
                return candidate
    return None


def find_artist_image_key(all_keys: Set[str], artist_name: str, normalized_keys: Optional[Dict[str, str]] = None) -> Optional[str]:
    if not artist_name or artist_name == "Unknown":
        return None

    clean_name = artist_name.strip()
    normalized_clean_name = normalize_name(clean_name)
    key_map = normalized_keys or build_normalized_key_map(all_keys)
    search_prefixes = ["", "artists/", "Artists/", "artist_images/", "images/"]

    for prefix in search_prefixes:
        for ext in IMAGE_EXTENSIONS:
            candidate = key_map.get(normalize_key(f"{prefix}{clean_name}{ext}"))
            if candidate:
                return candidate

    for key in all_keys:
        if not has_extension(key, IMAGE_EXTENSIONS):
            continue
        filename = key.rsplit("/", 1)[-1] if "/" in key else key
        stem, _ = os.path.splitext(filename)
        if normalize_name(stem) == normalized_clean_name:
            return key

    return None


def read_id3_tags_from_file(file_path: str, include_cover_data: bool = True) -> Dict[str, Optional[str]]:
    """Read ID3 tags from a local MP3 file."""
    title = os.path.basename(file_path)
    artist = "Unknown"
    album = "Unknown"
    duration = None
    cover_mime = None
    cover_b64 = None

    try:
        audio = MP3(file_path, ID3=ID3)
        tags = audio.tags

        if tags:
            try: title = tags["TIT2"].text[0]
            except: pass
            try: artist = tags["TPE1"].text[0]
            except: pass
            try: album = tags["TALB"].text[0]
            except: pass

        if audio.info and hasattr(audio.info, "length"):
            duration = round(float(audio.info.length), 2)

        if include_cover_data and INCLUDE_COVER and tags:
            apic_frames = tags.getall("APIC")
            if apic_frames:
                apic = apic_frames[0]
                data = apic.data
                if len(data) <= MAX_COVER_BYTES:
                    cover_mime = apic.mime
                    cover_b64 = base64.b64encode(data).decode("utf-8")

    except Exception as e:
        pass

    return {
        "title": title,
        "artist": artist,
        "album": album,
        "duration": duration,
        "cover_mime": cover_mime,
        "cover_b64": cover_b64,
    }


def generate_index_from_r2(r2_url: str = "") -> List[Dict]:
    """
    Generate music index by scanning R2 bucket contents.
    Downloads MP3 files temporarily to read ID3 tags.
    """
    # Step 1: List all objects in R2
    all_objects = list_r2_keys()
    all_keys_set = set(all_objects.keys())
    normalized_keys = build_normalized_key_map(all_keys_set)

    # Step 2: Filter MP3 files
    mp3_keys = sorted([k for k in all_objects if k.lower().endswith(".mp3")])
    print(f"[INFO] Found {len(mp3_keys)} MP3 files in R2.")

    index = []
    artist_image_cache = {}

    with tempfile.TemporaryDirectory(prefix="r2_music_") as tmp_dir:
        for i, mp3_key in enumerate(mp3_keys):
            if i % PROGRESS_EVERY == 0:
                print(f"Processing {i}/{len(mp3_keys)}...")

            # Derive paths
            mp3_stem = os.path.splitext(mp3_key)[0]  # e.g. "Artist/SongName"
            mp3_filename = mp3_key.rsplit("/", 1)[-1] if "/" in mp3_key else mp3_key
            mp3_filename_stem = os.path.splitext(mp3_filename)[0]
            dir_prefix = mp3_key.rsplit("/", 1)[0] if "/" in mp3_key else ""

            # Step 3: Download MP3 temporarily to read ID3 tags
            tmp_mp3 = os.path.join(tmp_dir, mp3_filename)
            # Use a unique subdir to avoid name collisions across directories
            tmp_subdir = os.path.join(tmp_dir, str(i))
            os.makedirs(tmp_subdir, exist_ok=True)
            tmp_mp3 = os.path.join(tmp_subdir, mp3_filename)

            if not download_partial_from_r2(mp3_key, tmp_mp3):
                print(f"[WARN] Skipping {mp3_key} (download failed)")
                continue

            # Read ID3 tags
            # When using R2 URL, prefer external cover files over embedded
            read_cover = not r2_url
            tags = read_id3_tags_from_file(tmp_mp3, include_cover_data=read_cover)

            # Clean up the temp file immediately to save disk space
            try:
                os.remove(tmp_mp3)
            except:
                pass

            artist_name = tags.get("artist")

            # --- Artist Image ---
            artist_image_web_path = None
            if artist_name and artist_name != "Unknown":
                artist_candidates = [artist_name] + [
                    name for name in split_artist_names(artist_name)
                    if name != artist_name
                ]
                for candidate_artist in artist_candidates:
                    if candidate_artist in artist_image_cache:
                        artist_image_web_path = artist_image_cache[candidate_artist]
                    else:
                        artist_key = find_artist_image_key(all_keys_set, candidate_artist, normalized_keys)
                        artist_image_web_path = make_web_path(artist_key, r2_url) if artist_key else None
                        artist_image_cache[candidate_artist] = artist_image_web_path
                    if artist_image_web_path:
                        break

            # --- Cover Art ---
            cover_val = None
            external_cover_key = find_cover_key(all_keys_set, dir_prefix, mp3_filename_stem, normalized_keys)

            if r2_url:
                # R2 URL mode: use URL to external cover
                if external_cover_key:
                    cover_val = make_web_path(external_cover_key, r2_url)
                elif tags["cover_b64"]:
                    cover_val = {
                        "format": tags["cover_mime"],
                        "data": tags["cover_b64"],
                    }
            else:
                # Relative path mode: prefer embedded, fallback to external
                if tags["cover_b64"]:
                    cover_val = {
                        "format": tags["cover_mime"],
                        "data": tags["cover_b64"],
                    }
                elif external_cover_key:
                    cover_val = make_web_path(external_cover_key, r2_url)

            # --- MP3 Path ---
            full_web_path = make_web_path(mp3_key, r2_url)

            # --- Video ---
            video_key = find_video_key(all_keys_set, mp3_stem, normalized_keys)
            video_web_path = make_web_path(video_key, r2_url) if video_key else None

            # --- LRC ---
            lrc_key = find_related_key(all_keys_set, mp3_stem, LYRIC_EXTENSIONS, normalized_keys)
            lrc_web_path = make_web_path(lrc_key, r2_url) if lrc_key else None

            # --- Date ---
            # Use R2 LastModified timestamp
            last_modified = all_objects[mp3_key].get("LastModified")
            date_val = 0
            if last_modified:
                date_val = last_modified.timestamp() * 1000

            entry = {
                "path": full_web_path,
                "title": tags["title"],
                "artist": tags["artist"],
                "album": tags["album"],
                "duration": tags["duration"],
                "date": date_val,
                "lrc": lrc_web_path,
                "video": video_web_path,
                "artistImage": artist_image_web_path,
                "cover": cover_val,
            }
            index.append(entry)

    return index


def scan_and_generate_index(_music_dir: str, output_json: str = OUTPUT_JSON, r2_url: str = "") -> List[Dict]:
    index = generate_index_from_r2(r2_url)
    with open(output_json, "w", encoding="utf-8") as f:
        json.dump(index, f, ensure_ascii=False, indent=2)
    return index


def main():
    print("=" * 60)
    print("🎵 Music Index Generator (Cloudflare R2)")
    print("=" * 60)

    r2_url = R2_PUBLIC_URL

    index = generate_index_from_r2(r2_url)

    try:
        with open(OUTPUT_JSON, "w", encoding="utf-8") as f:
            json.dump(index, f, ensure_ascii=False, indent=2)
        print(f"\n[SUCCESS] Index generated: {OUTPUT_JSON} ({len(index)} songs)")
    except Exception as e:
        print(f"\n[ERROR] Failed to save index: {e}")


if __name__ == "__main__":
    main()
