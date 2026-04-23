#!/usr/bin/env python3

import argparse
import os
import subprocess
import sys
import xml.etree.ElementTree as ET

from PIL import Image, ImageDraw, ImageFont


SVG_NS = {"svg": "http://www.w3.org/2000/svg"}
DEFAULT_SVG_PATH = "tmp/chemcore_glyph_kernel_preview.svg"
DEFAULT_PNG_PATH = "tmp/chemcore_glyph_kernel_preview.png"
REFERENCE_FONT_CANDIDATES = (
    os.environ.get("CHEMCORE_GLYPH_FONT"),
    "/usr/share/fonts/X11/Type1/qhvr.pfb",
)
LABEL_FONT_CANDIDATES = (
    "/usr/share/fonts/truetype/dejavu/DejaVuSans.ttf",
    "/usr/share/fonts/truetype/liberation2/LiberationSans-Regular.ttf",
)
EXPLICIT_SHAPES = {
    " ": "ShapeKind::kRect",
    "+": "ShapeKind::kRect",
    "-": "ShapeKind::kRect",
    "(": "ShapeKind::kRect",
    ")": "ShapeKind::kRect",
    "[": "ShapeKind::kRect",
    "]": "ShapeKind::kRect",
    ".": "ShapeKind::kRect",
    ",": "ShapeKind::kRect",
    "/": "ShapeKind::kRect",
    "∙": "ShapeKind::kEllipse",
    "•": "ShapeKind::kEllipse",
    **{glyph: ("ShapeKind::kEllipse" if glyph in "CGOQ" else "ShapeKind::kRect") for glyph in "ABCDEFGHIJKLMNOPQRSTUVWXYZ"},
    **{glyph: ("ShapeKind::kEllipse" if glyph in "cego" else "ShapeKind::kRect") for glyph in "abcdefghijklmnopqrstuvwxyz"},
    **{glyph: ("ShapeKind::kEllipse" if glyph in "0689" else "ShapeKind::kRect") for glyph in "0123456789"},
    "L": "ShapeKind::kRectCutTopRight",
    "h": "ShapeKind::kRectCutTopRight",
    "b": "ShapeKind::kRectCutTopRight",
    "P": "ShapeKind::kRectCutBottomRight",
    "F": "ShapeKind::kRectCutBottomRight",
    "d": "ShapeKind::kRectCutTopLeft",
    "q": "ShapeKind::kRectCutBottomLeft",
}


def resolve_font_path(user_path: str | None) -> str:
    candidates = []
    if user_path:
        candidates.append(user_path)
    candidates.extend(path for path in REFERENCE_FONT_CANDIDATES if path)
    for pattern in ("TeXGyreHeros", "Helvetica", "Arial"):
        try:
            output = subprocess.check_output(
                ["fc-match", "-f", "%{file}\n", pattern],
                text=True,
                stderr=subprocess.DEVNULL,
            ).strip()
        except Exception:
            output = ""
        if output:
            candidates.append(output)
    for path in candidates:
        if path and os.path.exists(path):
            return path
    raise FileNotFoundError("reference font not found; set --font or CHEMCORE_GLYPH_FONT")


def resolve_label_font() -> ImageFont.FreeTypeFont | ImageFont.ImageFont:
    for path in LABEL_FONT_CANDIDATES:
        if os.path.exists(path):
            return ImageFont.truetype(path, 18)
    return ImageFont.load_default()


def parse_float(value: str) -> float:
    return float(value.strip().replace("px", ""))


def parse_dimension(value: str, full_value: int) -> float:
    value = value.strip()
    if value.endswith("%"):
        return full_value * float(value[:-1]) / 100.0
    return parse_float(value)


def parse_path_points(value: str) -> list[tuple[float, float]]:
    tokens = value.replace(",", " ").split()
    points: list[tuple[float, float]] = []
    index = 0
    while index < len(tokens):
        command = tokens[index]
        index += 1
        if command == "Z":
            break
        if command not in {"M", "L"} or index + 1 >= len(tokens):
            raise ValueError(f"unsupported preview path command: {command}")
        points.append((float(tokens[index]), float(tokens[index + 1])))
        index += 2
    return points


def parse_svg(svg_path: str) -> tuple[int, int, list[dict]]:
    root = ET.parse(svg_path).getroot()
    width = int(round(parse_float(root.attrib["width"])))
    height = int(round(parse_float(root.attrib["height"])))
    ops: list[dict] = []

    for element in root:
        tag = element.tag.rsplit("}", 1)[-1]
        if tag == "rect":
            ops.append(
                {
                    "type": "rect",
                    "role": element.attrib.get("data-role", ""),
                    "x": parse_float(element.attrib.get("x", "0")),
                    "y": parse_float(element.attrib.get("y", "0")),
                    "width": parse_dimension(element.attrib.get("width", str(width)), width),
                    "height": parse_dimension(element.attrib.get("height", str(height)), height),
                    "fill": element.attrib.get("fill", "#ffffff"),
                }
            )
            continue
        if tag == "ellipse":
            ops.append(
                {
                    "type": "ellipse",
                    "role": element.attrib.get("data-role", ""),
                    "cx": parse_float(element.attrib["cx"]),
                    "cy": parse_float(element.attrib["cy"]),
                    "rx": parse_float(element.attrib["rx"]),
                    "ry": parse_float(element.attrib["ry"]),
                    "fill": element.attrib.get("fill", "#ffffff"),
                }
            )
            continue
        if tag == "path":
            ops.append(
                {
                    "type": "path",
                    "role": element.attrib.get("data-role", ""),
                    "points": parse_path_points(element.attrib["d"]),
                    "fill": element.attrib.get("fill", "#ffffff"),
                }
            )
            continue
        if tag == "text":
            ops.append(
                {
                    "type": "text",
                    "role": element.attrib.get("data-role", ""),
                    "x": parse_float(element.attrib["x"]),
                    "y": parse_float(element.attrib["y"]),
                    "font_size": parse_float(element.attrib.get("font-size", "16")),
                    "fill": element.attrib.get("fill", "#000000"),
                    "dominant_baseline": element.attrib.get("dominant-baseline", "alphabetic"),
                    "text": "".join(element.itertext()),
                }
            )
    return width, height, ops


def render_ops(svg_path: str, png_path: str, font_path: str, scale: int) -> None:
    width, height, ops = parse_svg(svg_path)
    image = Image.new("RGB", (width * scale, height * scale), "#050505")
    draw = ImageDraw.Draw(image)
    label_font = resolve_label_font()

    for op in ops:
        if op["type"] == "rect":
            box = (
                op["x"] * scale,
                op["y"] * scale,
                (op["x"] + op["width"]) * scale,
                (op["y"] + op["height"]) * scale,
            )
            draw.rectangle(box, fill=op["fill"])
            continue
        if op["type"] == "ellipse":
            box = (
                (op["cx"] - op["rx"]) * scale,
                (op["cy"] - op["ry"]) * scale,
                (op["cx"] + op["rx"]) * scale,
                (op["cy"] + op["ry"]) * scale,
            )
            draw.ellipse(box, fill=op["fill"])
            continue
        if op["type"] == "path":
            draw.polygon(
                [(x * scale, y * scale) for x, y in op["points"]],
                fill=op["fill"],
            )
            continue
        if op["role"] == "glyph-text":
            paste_glyph(image, font_path, op["text"], op["font_size"], op["x"], op["y"], scale)
            continue
        if op["role"] == "row-label":
            draw.text(
                (op["x"] * scale, op["y"] * scale),
                op["text"],
                font=label_font.font_variant(size=max(1, int(round(op["font_size"] * scale))))
                if hasattr(label_font, "font_variant")
                else label_font,
                fill=op["fill"],
                anchor="ls",
            )
            continue

    image.save(png_path)


def measure_bbox(font_path: str, text: str, font_size: float) -> tuple[float, float, float, float]:
    font = ImageFont.truetype(font_path, max(1, int(round(font_size))))
    bbox = font.getbbox(text, anchor="ls")
    return tuple(float(value) for value in bbox)


def paste_glyph(image: Image.Image, font_path: str, text: str, font_size: float, x: float, y: float, scale: int) -> None:
    font = ImageFont.truetype(font_path, max(1, int(round(font_size * scale))))
    bbox = font.getbbox(text, anchor="ls")
    mask = font.getmask(text, mode="L")
    mask_image = Image.frombytes("L", mask.size, bytes(mask))
    glyph = Image.new("RGB", mask.size, "#000000")
    image.paste(
        glyph,
        (
            int(round(x * scale + bbox[0])),
            int(round(y * scale + bbox[1])),
        ),
        mask_image,
    )


def check_glyphs(svg_path: str, font_path: str, tolerance: float) -> int:
    _, _, ops = parse_svg(svg_path)
    problems: list[str] = []
    shapes: list[dict] = []
    glyph_texts: list[dict] = []
    seen_glyph_text = False

    for op in ops:
        if op["role"] == "glyph-shape":
            if seen_glyph_text:
                problems.append("glyph shape appears after glyph text and may cover already-rendered text")
            shapes.append(op)
            continue
        if op["role"] == "glyph-text":
            seen_glyph_text = True
            glyph_texts.append(op)
            continue

    if len(shapes) != len(glyph_texts):
        problems.append(f"shape/text count mismatch: {len(shapes)} shapes, {len(glyph_texts)} texts")

    for shape, op in zip(shapes, glyph_texts):

        bbox = measure_bbox(font_path, op["text"], op["font_size"])
        glyph_box = (
            op["x"] + bbox[0],
            op["y"] + bbox[1],
            op["x"] + bbox[2],
            op["y"] + bbox[3],
        )
        if shape["type"] == "rect":
            shape_box = (
                shape["x"],
                shape["y"],
                shape["x"] + shape["width"],
                shape["y"] + shape["height"],
            )
        elif shape["type"] == "ellipse":
            shape_box = (
                shape["cx"] - shape["rx"],
                shape["cy"] - shape["ry"],
                shape["cx"] + shape["rx"],
                shape["cy"] + shape["ry"],
            )
        else:
            xs = [point[0] for point in shape["points"]]
            ys = [point[1] for point in shape["points"]]
            shape_box = (min(xs), min(ys), max(xs), max(ys))

        left_over = shape_box[0] - glyph_box[0]
        top_over = shape_box[1] - glyph_box[1]
        right_over = glyph_box[2] - shape_box[2]
        bottom_over = glyph_box[3] - shape_box[3]
        if (
            left_over > tolerance
            or top_over > tolerance
            or right_over > tolerance
            or bottom_over > tolerance
        ):
            problems.append(
                f"{op['text']!r} escapes shape: "
                f"left={left_over:.2f} top={top_over:.2f} right={right_over:.2f} bottom={bottom_over:.2f}"
            )

    if problems:
        for problem in problems:
            print(problem, file=sys.stderr)
        return 1
    print(f"glyph preview check passed for {svg_path} using {font_path}")
    return 0


def dump_profiles(font_path: str, font_size: int) -> None:
    font = ImageFont.truetype(font_path, font_size)
    for glyph, shape in EXPLICIT_SHAPES.items():
        if glyph == " ":
            advance = font.getlength(glyph) / font_size
            print(
                "{U' ', MakeProfile(U' ', ShapeKind::kRect, "
                + f"{advance:.2f}f, 0.00f, 0.00f, 0.00f, 0.00f, 0.00f, 0.00f, false)"
                + "},"
            )
            continue
        bbox = font.getbbox(glyph, anchor="ls")
        advance = font.getlength(glyph) / font_size
        print(
            f"{{U'{glyph}', MakeProfile(U'{glyph}', {shape}, {advance:.2f}f, "
            f"{bbox[0] / font_size:.2f}f, {bbox[1] / font_size:.2f}f, "
            f"{bbox[2] / font_size:.2f}f, {bbox[3] / font_size:.2f}f)"
            + "},"
        )


def main() -> int:
    parser = argparse.ArgumentParser(description="Reference-font tools for the chemcore glyph kernel")
    subparsers = parser.add_subparsers(dest="command", required=True)

    render_parser = subparsers.add_parser("render", help="render a deterministic PNG preview from kernel SVG output")
    render_parser.add_argument("--svg", default=DEFAULT_SVG_PATH)
    render_parser.add_argument("--png", default=DEFAULT_PNG_PATH)
    render_parser.add_argument("--font", default=None)
    render_parser.add_argument("--scale", type=int, default=2)

    check_parser = subparsers.add_parser("check", help="verify that rendered glyph bboxes stay inside kernel shapes")
    check_parser.add_argument("--svg", default=DEFAULT_SVG_PATH)
    check_parser.add_argument("--font", default=None)
    check_parser.add_argument("--tolerance", type=float, default=0.5)

    dump_parser = subparsers.add_parser("dump", help="dump suggested MakeProfile lines from the reference font")
    dump_parser.add_argument("--font", default=None)
    dump_parser.add_argument("--font-size", type=int, default=100)

    args = parser.parse_args()
    font_path = resolve_font_path(getattr(args, "font", None))

    if args.command == "render":
        render_ops(args.svg, args.png, font_path, args.scale)
        print(f"wrote {args.png} using {font_path}")
        return 0
    if args.command == "check":
        return check_glyphs(args.svg, font_path, args.tolerance)
    if args.command == "dump":
        dump_profiles(font_path, args.font_size)
        return 0
    return 1


if __name__ == "__main__":
    raise SystemExit(main())
