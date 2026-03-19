#!/usr/bin/env python3
"""Sprite generation pipeline: Imagen 3 (Vertex AI) → Gemini eval.

Usage:
    python3 tools/vision/sprite_gen.py --category enemies --batch-size 10
    python3 tools/vision/sprite_gen.py --category djinn --batch-size 5
    python3 tools/vision/sprite_gen.py --category enemies --start-index 20 --batch-size 10
    python3 tools/vision/sprite_gen.py --list  # show manifest + progress

Env vars required:
    GEMINI_OAUTH_CLIENT_ID, GEMINI_OAUTH_CLIENT_SECRET,
    GEMINI_OAUTH_REFRESH_TOKEN, GEMINI_VERTEX_PROJECT
"""

import argparse
import base64
import json
import os
import sys
import time
import urllib.error
import urllib.parse
import urllib.request

# ---------------------------------------------------------------------------
# Auth
# ---------------------------------------------------------------------------
_token_cache = {"token": None, "expires": 0}

def _get_token():
    if _token_cache["token"] and time.time() < _token_cache["expires"] - 60:
        return _token_cache["token"]
    data = urllib.parse.urlencode({
        "client_id": os.environ["GEMINI_OAUTH_CLIENT_ID"],
        "client_secret": os.environ["GEMINI_OAUTH_CLIENT_SECRET"],
        "grant_type": "refresh_token",
        "refresh_token": os.environ["GEMINI_OAUTH_REFRESH_TOKEN"],
    }).encode()
    resp = json.loads(urllib.request.urlopen(
        urllib.request.Request("https://oauth2.googleapis.com/token", data=data)
    ).read())
    _token_cache["token"] = resp["access_token"]
    _token_cache["expires"] = time.time() + resp.get("expires_in", 3600)
    return _token_cache["token"]

# ---------------------------------------------------------------------------
# Imagen 3 generation
# ---------------------------------------------------------------------------
ELEMENT_PALETTE = {
    "Venus": "earthy brown and green tones",
    "Mars": "fiery red and orange tones",
    "Mercury": "icy blue and cyan tones",
    "Jupiter": "purple and electric violet tones",
}

TIER_DESC = {
    1: "small, simple creature",
    2: "medium-sized, moderately detailed creature",
    3: "large, imposing and detailed creature",
    4: "massive, epic boss-tier creature with dramatic presence",
}

def build_prompt(entry, category):
    """Build Imagen prompt from manifest entry."""
    name = entry["name"]
    element = entry.get("element", "Venus")
    palette = ELEMENT_PALETTE.get(element, "neutral tones")

    if category == "enemies":
        tier = entry.get("tier", 1)
        tier_desc = TIER_DESC.get(tier, TIER_DESC[1])
        return (
            f"2D pixel art battle sprite of '{name}', a {tier_desc} from a classic JRPG. "
            f"{palette}. Front-facing battle pose. Clean pixel art style similar to Golden Sun GBA. "
            f"Single character centered on solid white background. No text, no UI elements. "
            f"Sharp pixel edges, limited color palette, 16-bit era aesthetic."
        )
    else:  # djinn
        return (
            f"2D pixel art sprite of '{name}', a small elemental djinn creature from a JRPG. "
            f"{palette}. Cute but magical appearance. Clean pixel art style similar to Golden Sun GBA. "
            f"Single small creature centered on solid white background. No text, no UI. "
            f"Sharp pixel edges, limited color palette, 16-bit era aesthetic. Glowing elemental aura."
        )

def generate_sprite(prompt, retries=3):
    """Call Imagen 3 via Vertex AI. Returns PNG bytes or None."""
    token = _get_token()
    project = os.environ["GEMINI_VERTEX_PROJECT"]
    url = (
        f"https://us-central1-aiplatform.googleapis.com/v1/projects/{project}"
        f"/locations/us-central1/publishers/google/models/imagen-3.0-generate-002:predict"
    )
    body = json.dumps({
        "instances": [{"prompt": prompt}],
        "parameters": {
            "sampleCount": 1,
            "aspectRatio": "1:1",
            "outputOptions": {"mimeType": "image/png"},
        },
    }).encode()

    for attempt in range(retries):
        try:
            req = urllib.request.Request(
                url, data=body,
                headers={"Authorization": f"Bearer {token}", "Content-Type": "application/json"},
            )
            resp = json.loads(urllib.request.urlopen(req, timeout=60).read())
            b64 = resp["predictions"][0]["bytesBase64Encoded"]
            return base64.b64decode(b64)
        except urllib.error.HTTPError as e:
            err_body = e.read().decode()[:200]
            if e.code == 429:
                wait = 2 ** (attempt + 1)
                print(f"  429 rate limit, waiting {wait}s...", flush=True)
                time.sleep(wait)
            elif e.code == 400 and "SAFETY" in err_body.upper():
                print(f"  SAFETY block for prompt, skipping", flush=True)
                return None
            else:
                print(f"  HTTP {e.code}: {err_body}", flush=True)
                if attempt < retries - 1:
                    time.sleep(2)
        except Exception as e:
            print(f"  Error: {e}", flush=True)
            if attempt < retries - 1:
                time.sleep(2)
    return None

# ---------------------------------------------------------------------------
# Gemini evaluation
# ---------------------------------------------------------------------------
def evaluate_sprite(png_path, entry, category):
    """Evaluate sprite quality with Gemini vision. Returns (score, verdict, reason)."""
    try:
        token = _get_token()
        project = os.environ["GEMINI_VERTEX_PROJECT"]

        with open(png_path, "rb") as f:
            b64 = base64.b64encode(f.read()).decode()

        name = entry["name"]
        element = entry.get("element", "Unknown")
        prompt = (
            f"Rate this JRPG battle sprite of '{name}' ({element} element) on a scale of 1-10. "
            f"Criteria: pixel art quality, character design clarity, element theming, "
            f"suitability for a Golden Sun-style battle scene. "
            f"Score 7+ = PASS, below 7 = REDO."
        )

        url = (
            f"https://aiplatform.googleapis.com/v1/projects/{project}"
            f"/locations/global/publishers/google/models/gemini-3-flash-preview:generateContent"
        )
        body = json.dumps({
            "contents": [{"role": "user", "parts": [
                {"inlineData": {"mimeType": "image/png", "data": b64}},
                {"text": prompt},
            ]}],
            "generationConfig": {
                "responseMimeType": "application/json",
                "responseSchema": {
                    "type": "OBJECT",
                    "properties": {
                        "score": {"type": "INTEGER"},
                        "verdict": {"type": "STRING", "enum": ["PASS", "REDO"]},
                        "reason": {"type": "STRING"},
                    },
                    "required": ["score", "verdict", "reason"],
                },
            },
        }).encode()

        req = urllib.request.Request(
            url, data=body,
            headers={"Authorization": f"Bearer {token}", "Content-Type": "application/json"},
        )
        resp = json.loads(urllib.request.urlopen(req, timeout=30).read())

        # Handle thinking models: iterate parts for text without thoughtSignature
        parts = resp["candidates"][0]["content"]["parts"]
        text = None
        for p in parts:
            if "text" in p and "thoughtSignature" not in p:
                text = p["text"]
                break
        if text is None:
            text = parts[0].get("text", "{}")

        result = json.loads(text)
        return result.get("score", 0), result.get("verdict", "REDO"), result.get("reason", "")
    except Exception as e:
        print(f"  Eval error: {e}", flush=True)
        return 0, "SKIP", str(e)

# ---------------------------------------------------------------------------
# Main pipeline
# ---------------------------------------------------------------------------
def get_output_path(entry, category, repo_root):
    """Get output path for a sprite."""
    filename = entry["id"].replace("-", "_") + ".png"
    if category == "enemies":
        return os.path.join(repo_root, "assets/sprites/battle/enemies/generated", filename)
    else:
        return os.path.join(repo_root, "assets/sprites/battle/djinn/generated", filename)

def run_batch(manifest_path, category, batch_size, start_index, repo_root, eval_sprites=True, redo_mode=False):
    """Generate a batch of sprites."""
    with open(manifest_path) as f:
        manifest = json.load(f)

    entries = manifest.get(category, [])
    if not entries:
        print(f"No entries for category '{category}'")
        return

    if redo_mode:
        # Find sprites that scored below 7 in results log
        results_path = os.path.join(repo_root, f"status/sprite-results-{category}.jsonl")
        redo_ids = set()
        if os.path.exists(results_path):
            with open(results_path) as f:
                for line in f:
                    r = json.loads(line)
                    if r.get("status") == "REDO" or (r.get("score", 10) < 7 and r.get("status") != "FAILED"):
                        redo_ids.add(r["id"])
        pending = [e for e in entries if e["id"] in redo_ids]
        # Delete existing PNGs so they get regenerated
        for entry in pending:
            out_path = get_output_path(entry, category, repo_root)
            if os.path.exists(out_path):
                os.remove(out_path)
        print(f"=== REDO MODE: {len(pending)} sprites to regenerate ===", flush=True)
    else:
        # Filter to entries that don't have PNGs yet
        pending = []
        for entry in entries:
            out_path = get_output_path(entry, category, repo_root)
            if not os.path.exists(out_path):
                pending.append(entry)

    total = len(entries)
    done = total - len(pending)
    print(f"=== {category.upper()} SPRITES: {done}/{total} done, {len(pending)} pending ===", flush=True)

    if not pending:
        print("All sprites generated!")
        return

    # Apply start_index and batch_size
    batch = pending[start_index:start_index + batch_size]
    if not batch:
        print(f"No entries in range [start={start_index}, batch={batch_size}]")
        return

    print(f"Generating batch of {len(batch)} (indices {start_index}-{start_index+len(batch)-1})", flush=True)

    results_path = os.path.join(repo_root, f"status/sprite-results-{category}.jsonl")
    
    generated = 0
    passed = 0
    for i, entry in enumerate(batch):
        out_path = get_output_path(entry, category, repo_root)
        os.makedirs(os.path.dirname(out_path), exist_ok=True)
        
        prompt = build_prompt(entry, category)
        print(f"[{i+1}/{len(batch)}] {entry['name']} ({entry['element']})...", end=" ", flush=True)

        # Rate limit: 8s minimum between Imagen calls (Vertex AI ~5-10 RPM)
        if i > 0:
            time.sleep(15)

        t0 = time.time()
        png_bytes = generate_sprite(prompt)
        gen_time = time.time() - t0

        if png_bytes is None:
            print(f"FAILED ({gen_time:.1f}s)", flush=True)
            result = {"id": entry["id"], "name": entry["name"], "status": "FAILED", "gen_time": gen_time}
        else:
            with open(out_path, "wb") as f:
                f.write(png_bytes)
            size_kb = len(png_bytes) / 1024
            generated += 1

            if eval_sprites:
                score, verdict, reason = evaluate_sprite(out_path, entry, category)
                print(f"{verdict} (score={score}, {size_kb:.0f}KB, {gen_time:.1f}s) — {reason[:60]}", flush=True)
                if verdict == "PASS":
                    passed += 1
                result = {
                    "id": entry["id"], "name": entry["name"], "status": verdict,
                    "score": score, "reason": reason, "size_kb": round(size_kb),
                    "gen_time": round(gen_time, 1), "path": out_path,
                }
            else:
                print(f"OK ({size_kb:.0f}KB, {gen_time:.1f}s)", flush=True)
                result = {
                    "id": entry["id"], "name": entry["name"], "status": "OK",
                    "size_kb": round(size_kb), "gen_time": round(gen_time, 1), "path": out_path,
                }

        # Append to results log
        with open(results_path, "a") as f:
            f.write(json.dumps(result) + "\n")

    now_done = done + generated
    print(f"\n=== BATCH COMPLETE: {generated}/{len(batch)} generated, {passed} passed ===", flush=True)
    print(f"=== TOTAL PROGRESS: {now_done}/{total} ===", flush=True)

def list_progress(manifest_path, repo_root):
    """Show current progress."""
    with open(manifest_path) as f:
        manifest = json.load(f)

    for cat in ["enemies", "djinn"]:
        entries = manifest.get(cat, [])
        done = sum(1 for e in entries if os.path.exists(get_output_path(e, cat, repo_root)))
        print(f"{cat}: {done}/{len(entries)}")
        if done < len(entries):
            pending = [e["name"] for e in entries if not os.path.exists(get_output_path(e, cat, repo_root))]
            print(f"  Next pending: {', '.join(pending[:5])}{'...' if len(pending) > 5 else ''}")

def main():
    parser = argparse.ArgumentParser(description="Sprite generation pipeline")
    parser.add_argument("--manifest", default="status/sprite-manifest.json")
    parser.add_argument("--category", choices=["enemies", "djinn"], default="enemies")
    parser.add_argument("--batch-size", type=int, default=10)
    parser.add_argument("--start-index", type=int, default=0)
    parser.add_argument("--no-eval", action="store_true", help="Skip Gemini evaluation")
    parser.add_argument("--redo", action="store_true", help="Regenerate sprites that scored below 7")
    parser.add_argument("--list", action="store_true", help="Show progress and exit")
    parser.add_argument("--repo-root", default=".")
    args = parser.parse_args()

    if args.list:
        list_progress(args.manifest, args.repo_root)
        return

    run_batch(
        args.manifest, args.category, args.batch_size, args.start_index,
        args.repo_root, eval_sprites=not args.no_eval, redo_mode=args.redo,
    )

if __name__ == "__main__":
    main()
