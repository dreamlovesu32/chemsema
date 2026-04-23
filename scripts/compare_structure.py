from __future__ import annotations

from pathlib import Path
import cv2
import numpy as np


ROOT = Path("/home/jiajun/chemcore")
TMP = ROOT / "tmp"

REFERENCE_IMAGE = ROOT / "a2bd4f1ad2929bbca7fd45c770908533.jpg"
CURRENT_IMAGE = TMP / "viewer-svg.png"

# These crops isolate the chemistry region and avoid UI chrome.
REFERENCE_CROP = (300, 40, 1300, 260)
CURRENT_CROP = (240, 130, 1390, 410)


def load_gray(path: Path) -> np.ndarray:
    image = cv2.imread(str(path), cv2.IMREAD_GRAYSCALE)
    if image is None:
        raise FileNotFoundError(path)
    return image


def crop(image: np.ndarray, box: tuple[int, int, int, int]) -> np.ndarray:
    x1, y1, x2, y2 = box
    return image[y1:y2, x1:x2].copy()


def preprocess(image: np.ndarray) -> np.ndarray:
    blur = cv2.GaussianBlur(image, (3, 3), 0)
    _, thresh = cv2.threshold(blur, 235, 255, cv2.THRESH_BINARY_INV)
    return thresh


def align(reference: np.ndarray, current: np.ndarray) -> tuple[np.ndarray, np.ndarray]:
    ref_h, ref_w = reference.shape
    cur_h, cur_w = current.shape
    current_scaled = cv2.resize(current, (ref_w, ref_h), interpolation=cv2.INTER_CUBIC)

    warp = np.eye(2, 3, dtype=np.float32)
    criteria = (
        cv2.TERM_CRITERIA_EPS | cv2.TERM_CRITERIA_COUNT,
        300,
        1e-6,
    )
    try:
        cv2.findTransformECC(
            preprocess(reference).astype(np.float32) / 255.0,
            preprocess(current_scaled).astype(np.float32) / 255.0,
            warp,
            cv2.MOTION_AFFINE,
            criteria,
            None,
            5,
        )
    except cv2.error:
        pass

    aligned = cv2.warpAffine(
        current_scaled,
        warp,
        (ref_w, ref_h),
        flags=cv2.INTER_LINEAR | cv2.WARP_INVERSE_MAP,
        borderMode=cv2.BORDER_CONSTANT,
        borderValue=255,
    )
    return aligned, warp


def colorize_diff(reference: np.ndarray, aligned: np.ndarray) -> np.ndarray:
    ref_inv = preprocess(reference)
    cur_inv = preprocess(aligned)
    missing = cv2.subtract(ref_inv, cur_inv)
    extra = cv2.subtract(cur_inv, ref_inv)

    canvas = np.full((reference.shape[0], reference.shape[1], 3), 255, dtype=np.uint8)
    canvas[:, :, 0] = np.where(extra > 0, 255 - extra // 2, canvas[:, :, 0])
    canvas[:, :, 1] = np.where((missing > 0) | (extra > 0), 235, canvas[:, :, 1])
    canvas[:, :, 2] = np.where(missing > 0, 255 - missing // 2, canvas[:, :, 2])
    return canvas


def overlay(reference: np.ndarray, aligned: np.ndarray) -> np.ndarray:
    ref_rgb = cv2.cvtColor(reference, cv2.COLOR_GRAY2BGR)
    aligned_rgb = cv2.cvtColor(aligned, cv2.COLOR_GRAY2BGR)
    return cv2.addWeighted(ref_rgb, 0.5, aligned_rgb, 0.5, 0)


def main() -> None:
    reference = crop(load_gray(REFERENCE_IMAGE), REFERENCE_CROP)
    current = crop(load_gray(CURRENT_IMAGE), CURRENT_CROP)
    aligned, warp = align(reference, current)

    cv2.imwrite(str(TMP / "structure-ref-crop.png"), reference)
    cv2.imwrite(str(TMP / "structure-current-crop.png"), current)
    cv2.imwrite(str(TMP / "structure-current-aligned.png"), aligned)
    cv2.imwrite(str(TMP / "structure-diff-overlay.png"), overlay(reference, aligned))
    cv2.imwrite(str(TMP / "structure-diff-heat.png"), colorize_diff(reference, aligned))
    print("warp", warp.tolist())
    print(TMP / "structure-diff-overlay.png")
    print(TMP / "structure-diff-heat.png")


if __name__ == "__main__":
    main()
