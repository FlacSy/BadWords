"""Locating and fetching the ONNX toxicity model."""

from __future__ import annotations

import hashlib
import json
import os
import shutil
import tempfile
import urllib.request
import zipfile
from pathlib import Path

GITHUB_OWNER = "FlacSy"
GITHUB_REPO = "badwords"
ASSET_NAME = "badwords-ml-model.zip"

#: Files a usable model directory must contain. Checking only model.onnx let a
#: half-extracted download look valid and fail later inside the tokenizer.
REQUIRED_FILES = (
    "model.onnx",
    "config.json",
    "tokenizer.json",
    "tokenizer_config.json",
)

_CHUNK = 1 << 20
_API_TIMEOUT = 30
_DOWNLOAD_TIMEOUT = 300


class ModelNotFoundError(RuntimeError):
    """No usable model directory could be located."""


class ModelDownloadError(RuntimeError):
    """The model could not be downloaded or verified."""


def cache_dir() -> Path:
    """Directory the downloaded model is cached in."""
    base = os.environ.get("XDG_CACHE_HOME") or Path.home() / ".cache"
    return Path(base) / "badwords" / "ml"


def is_complete(path: Path) -> bool:
    """Whether a directory holds every file the model needs."""
    return all((path / name).exists() for name in REQUIRED_FILES)


def _repo_models_dir() -> Path | None:
    """`ml/models` in a source checkout, if it holds a model."""
    candidate = Path(__file__).resolve().parents[3] / "ml" / "models"
    return candidate if is_complete(candidate) else None


def get_model_dir(*, download: bool = True) -> Path:
    """Locate the model, downloading it if necessary.

    Resolution order:

    1. ``BADWORDS_ML_PATH``
    2. ``ml/models`` when running from a source checkout
    3. the cache directory
    4. the GitHub release asset, downloaded into the cache

    :param download: When false, raise instead of reaching for the network.
    :raises ModelNotFoundError: If no model is available.
    :raises ModelDownloadError: If the download fails or is incomplete.
    """
    override = os.environ.get("BADWORDS_ML_PATH")
    if override:
        path = Path(override).resolve()
        if is_complete(path):
            return path
        missing = [name for name in REQUIRED_FILES if not (path / name).exists()]
        message = f"BADWORDS_ML_PATH={override} is missing: {', '.join(missing)}"
        raise ModelNotFoundError(message)

    repo_models = _repo_models_dir()
    if repo_models is not None:
        return repo_models

    cached = cache_dir() / "model"
    if is_complete(cached):
        return cached

    if not download:
        message = (
            f"no model in {cached}. Call badwords.ml.download_model(), set "
            f"BADWORDS_ML_PATH, or allow the download."
        )
        raise ModelNotFoundError(message)

    return download_model()


def download_model(*, force: bool = False, tag: str | None = None) -> Path:
    """Download the model into the cache and return its directory.

    Streams to a temporary directory and moves it into place only once every
    required file is present, so an interrupted download cannot leave a broken
    model cached forever.

    :param force: Re-download even if a complete model is already cached.
    :param tag: Release tag to pin. Defaults to the latest release.
    :raises ModelDownloadError: If the download fails or the archive is incomplete.
    """
    target = cache_dir() / "model"
    if is_complete(target) and not force:
        return target

    asset_url, expected_size = _find_asset(tag)
    cache_dir().mkdir(parents=True, exist_ok=True)

    with tempfile.TemporaryDirectory(dir=cache_dir(), prefix=".download-") as tmp:
        tmp_path = Path(tmp)
        archive = tmp_path / ASSET_NAME
        digest = _stream_download(asset_url, archive)

        if expected_size and archive.stat().st_size != expected_size:
            message = f"downloaded {archive.stat().st_size} bytes, expected {expected_size}"
            raise ModelDownloadError(message)

        extracted = tmp_path / "model"
        _safe_extract(archive, extracted)
        _flatten_single_subdirectory(extracted)

        missing = [name for name in REQUIRED_FILES if not (extracted / name).exists()]
        if missing:
            message = f"{ASSET_NAME} (sha256 {digest}) is missing: {', '.join(missing)}"
            raise ModelDownloadError(message)

        if target.exists():
            shutil.rmtree(target)
        shutil.move(str(extracted), str(target))

    return target


def _find_asset(tag: str | None) -> tuple[str, int | None]:
    """URL and expected size of the model asset in a release."""
    if tag:
        api = f"https://api.github.com/repos/{GITHUB_OWNER}/{GITHUB_REPO}/releases/tags/{tag}"
    else:
        api = f"https://api.github.com/repos/{GITHUB_OWNER}/{GITHUB_REPO}/releases/latest"

    request = urllib.request.Request(  # noqa: S310 - fixed https URL
        api,
        headers={"Accept": "application/vnd.github+json"},
    )
    try:
        with urllib.request.urlopen(request, timeout=_API_TIMEOUT) as response:  # noqa: S310
            release = json.loads(response.read().decode())
    except OSError as exc:
        message = f"cannot reach the GitHub release API: {exc}"
        raise ModelDownloadError(message) from exc

    for asset in release.get("assets", []):
        if asset.get("name") == ASSET_NAME:
            return asset["browser_download_url"], asset.get("size")

    release_name = release.get("tag_name", tag or "latest")
    message = (
        f"release {release_name} has no {ASSET_NAME}. Upload the model or set BADWORDS_ML_PATH."
    )
    raise ModelDownloadError(message)


def _stream_download(url: str, destination: Path) -> str:
    """Download to a file in chunks, returning the sha256 of what arrived."""
    digest = hashlib.sha256()
    try:
        with (
            urllib.request.urlopen(url, timeout=_DOWNLOAD_TIMEOUT) as response,  # noqa: S310
            destination.open("wb") as out,
        ):
            while chunk := response.read(_CHUNK):
                digest.update(chunk)
                out.write(chunk)
    except OSError as exc:
        message = f"download failed: {exc}"
        raise ModelDownloadError(message) from exc
    return digest.hexdigest()


def _safe_extract(archive: Path, destination: Path) -> None:
    """Extract a zip, refusing entries that escape the destination."""
    destination.mkdir(parents=True, exist_ok=True)
    resolved_destination = destination.resolve()

    with zipfile.ZipFile(archive) as zf:
        for member in zf.infolist():
            target = (destination / member.filename).resolve()
            if not target.is_relative_to(resolved_destination):
                message = f"{ASSET_NAME} contains an unsafe path: {member.filename}"
                raise ModelDownloadError(message)
        zf.extractall(destination)


def _flatten_single_subdirectory(path: Path) -> None:
    """Move files up if the archive wrapped everything in one directory."""
    if is_complete(path):
        return
    entries = list(path.iterdir())
    if len(entries) != 1 or not entries[0].is_dir():
        return
    inner = entries[0]
    for item in inner.iterdir():
        item.rename(path / item.name)
    inner.rmdir()
