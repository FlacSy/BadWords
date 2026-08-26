"""Model path resolution: env, local dev, cache, GitHub Releases download."""

from __future__ import annotations

import os
import zipfile
from pathlib import Path

# GitHub repo for model releases
GITHUB_OWNER = "FlacSy"
GITHUB_REPO = "badwords"
ASSET_NAME = "badwords-ml-model.zip"


def _cache_dir() -> Path:
    """User cache directory for ML model."""
    base = os.environ.get("XDG_CACHE_HOME") or os.path.expanduser("~/.cache")
    return Path(base) / "badwords" / "ml"


def _dev_models_dir() -> Path | None:
    """Local ml/models for development (repo root)."""
    # From python/badwords/ml/_paths.py -> repo root is 4 levels up
    p = Path(__file__).resolve().parent.parent.parent.parent / "ml" / "models"
    return p if (p / "model.onnx").exists() else None


def get_model_dir() -> Path:
    """Resolve model directory. Downloads from GitHub Releases if needed.

    Priority:
    1. BADWORDS_ML_PATH env
    2. Local ml/models (dev)
    3. Cache (~/.cache/badwords/ml)
    4. Download from GitHub Releases -> cache
    """
    if path := os.environ.get("BADWORDS_ML_PATH"):
        p = Path(path).resolve()
        if (p / "model.onnx").exists():
            return p
        raise FileNotFoundError(f"BADWORDS_ML_PATH={path} has no model.onnx")

    if dev := _dev_models_dir():
        return dev

    cache = _cache_dir()
    model_dir = cache / "model"
    if (model_dir / "model.onnx").exists():
        return model_dir

    # Download from GitHub Releases
    _download_model(cache)
    return model_dir


def _download_model(cache_dir: Path) -> None:
    """Download model from GitHub Releases and extract to cache."""
    import json
    import urllib.request

    cache_dir.mkdir(parents=True, exist_ok=True)
    zip_path = cache_dir / ASSET_NAME

    # Get latest release
    api_url = f"https://api.github.com/repos/{GITHUB_OWNER}/{GITHUB_REPO}/releases/latest"
    req = urllib.request.Request(api_url, headers={"Accept": "application/vnd.github+json"})
    with urllib.request.urlopen(req, timeout=30) as r:
        release = json.loads(r.read().decode())

    # Find asset
    asset = next((a for a in release.get("assets", []) if a["name"] == ASSET_NAME), None)
    if not asset:
        raise FileNotFoundError(
            f"Asset {ASSET_NAME} not found in release {release.get('tag_name', '?')}. "
            "Upload model to GitHub Releases or set BADWORDS_ML_PATH."
        )

    url = asset["browser_download_url"]
    # Download
    with urllib.request.urlopen(urllib.request.Request(url), timeout=120) as r:
        zip_path.write_bytes(r.read())

    # Extract
    model_dir = cache_dir / "model"
    model_dir.mkdir(exist_ok=True)
    with zipfile.ZipFile(zip_path, "r") as zf:
        zf.extractall(model_dir)

    zip_path.unlink(missing_ok=True)

    if not (model_dir / "model.onnx").exists():
        # Maybe extracted to subdir
        for sub in model_dir.iterdir():
            if sub.is_dir() and (sub / "model.onnx").exists():
                for f in sub.iterdir():
                    f.rename(model_dir / f.name)
                sub.rmdir()
                break
