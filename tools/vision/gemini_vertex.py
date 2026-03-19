"""Gemini access via Vertex AI (bypasses blocked consumer endpoint).

Stable models (2.x) → us-central1
Preview models (3.x) → global

Functions:
    gemini_generate(prompt, model, system) → str
    gemini_vision(image_path, prompt, model) → str
    gemini_generate_json(prompt, schema, model) → dict
    gemini_vision_json(image_path, prompt, schema, model) → dict
"""

import base64
import json
import os
import time
import urllib.request
import urllib.parse

# Token cache
_token_cache = {"token": None, "expires": 0}

LOCATION_MAP = {
    "gemini-2.5-flash": "us-central1",
    "gemini-2.5-pro": "us-central1",
    "gemini-3-flash-preview": "global",
    "gemini-3.1-pro-preview": "global",
    "gemini-3.1-flash-lite-preview": "global",
}

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


def _get_endpoint(model):
    location = LOCATION_MAP.get(model, "global")
    project = os.environ["GEMINI_VERTEX_PROJECT"]
    if location == "global":
        return f"https://aiplatform.googleapis.com/v1/projects/{project}/locations/global/publishers/google/models/{model}:generateContent"
    else:
        return f"https://{location}-aiplatform.googleapis.com/v1/projects/{project}/locations/{location}/publishers/google/models/{model}:generateContent"


def _call(contents, model="gemini-3-flash-preview", generation_config=None, system=None):
    token = _get_token()
    body = {"contents": contents}
    if generation_config:
        body["generationConfig"] = generation_config
    if system:
        body["systemInstruction"] = {"parts": [{"text": system}]}

    req = urllib.request.Request(
        _get_endpoint(model),
        data=json.dumps(body).encode(),
        headers={"Authorization": f"Bearer {token}", "Content-Type": "application/json"},
    )
    resp = json.loads(urllib.request.urlopen(req).read())
    return resp["candidates"][0]["content"]["parts"][0]["text"]


def gemini_generate(prompt, model="gemini-3-flash-preview", system=None):
    """Text generation."""
    contents = [{"role": "user", "parts": [{"text": prompt}]}]
    return _call(contents, model=model, system=system)


def gemini_vision(image_path, prompt, model="gemini-3-flash-preview"):
    """Image + prompt → text response."""
    with open(image_path, "rb") as f:
        b64 = base64.b64encode(f.read()).decode()

    ext = image_path.rsplit(".", 1)[-1].lower()
    mime = {"png": "image/png", "jpg": "image/jpeg", "jpeg": "image/jpeg",
            "gif": "image/gif", "webp": "image/webp"}.get(ext, "image/png")

    contents = [{"role": "user", "parts": [
        {"inlineData": {"mimeType": mime, "data": b64}},
        {"text": prompt},
    ]}]
    return _call(contents, model=model)


def gemini_generate_json(prompt, schema=None, model="gemini-3-flash-preview", system=None):
    """Text generation with forced JSON output."""
    gen_config = {"responseMimeType": "application/json"}
    if schema:
        gen_config["responseSchema"] = schema
    contents = [{"role": "user", "parts": [{"text": prompt}]}]
    result = _call(contents, model=model, generation_config=gen_config, system=system)
    return json.loads(result)


def gemini_vision_json(image_path, prompt, schema=None, model="gemini-3-flash-preview"):
    """Image + prompt → forced JSON response."""
    with open(image_path, "rb") as f:
        b64 = base64.b64encode(f.read()).decode()

    ext = image_path.rsplit(".", 1)[-1].lower()
    mime = {"png": "image/png", "jpg": "image/jpeg", "jpeg": "image/jpeg",
            "gif": "image/gif", "webp": "image/webp"}.get(ext, "image/png")

    gen_config = {"responseMimeType": "application/json"}
    if schema:
        gen_config["responseSchema"] = schema

    contents = [{"role": "user", "parts": [
        {"inlineData": {"mimeType": mime, "data": b64}},
        {"text": prompt},
    ]}]
    result = _call(contents, model=model, generation_config=gen_config)
    return json.loads(result)
