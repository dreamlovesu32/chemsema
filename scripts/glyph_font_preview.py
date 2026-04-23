#!/usr/bin/env python3

import argparse
import html
import math
import os
import subprocess
from dataclasses import dataclass
from typing import Iterable

from PIL import Image, ImageDraw, ImageFont


ELLIPSE_CHARS = set("CGOQcego0689")
DESCENDER_CHARS = set("JQgjpqy")
ALIGN_VALUES = {"right", "left", "above", "below"}
SHEET_SECTIONS = [
    (
        "Core atoms and symbols",
        [
            "C",
            "N",
            "O",
            "H",
            "S",
            "P",
            "F",
            "Cl",
            "Br",
            "I",
            "B",
            "Si",
            "Ca",
        ],
    ),
    (
        "Scripts and charges",
        [
            "N^3",
            "H_2O",
            "CO_2",
            "SO_2",
            "O_2S@2#left",
            "SO_2#above",
            "SO_2#below",
            "NTs#above",
            "NTs#below",
            "Mg^2+",
            "Ca^2+",
            "SO_4^2-",
            "PO_4^3-",
            "NH_4^+",
            "CF_3",
            "NO_2",
            "O_2",
            "C_6H_5",
            "CH_3",
        ],
    ),
    (
        "Common fragments",
        [
            "Ph",
            "HN",
            "CN",
            "Me",
            "Et",
            "OMe",
            "NMe_2",
            "CO_2Me",
            "t-Bu",
            "n-Bu",
            "i-Pr",
            "CH_2OH",
            "Ca@1",
            "Br@1",
        ],
    ),
    (
        "Stress cases",
        [
            "Boc",
            "Ts",
            "Ac",
            "OPh",
            "NHBoc",
            "COCl",
            "SO_2NH_2",
            "CONH_2",
            "CF_3SO_2",
            "P(O)(OEt)_2",
            "CCl_3",
            "CBr_3",
        ],
    ),
    (
        "Single-glyph calibration",
        [
            "ABCDEF",
            "GHIJKL",
            "MNOPQR",
            "STUVWX",
            "YZ",
            "abcdef",
            "ghijkl",
            "mnopqr",
            "stuvwx",
            "yz",
            "012345",
            "6789",
            "-+()[]",
            "=#.,",
            "/\\\\",
            "[]()",
        ],
    ),
]


@dataclass(frozen=True)
class Token:
    char: str
    script: str


@dataclass
class GlyphPlacement:
    char: str
    script: str
    x: float
    baseline_y: float
    font_size: float
    advance: float
    bbox: tuple[float, float, float, float]
    shape: str
    background: tuple[float, float, float, float]


@dataclass
class TextPlacement:
    text: str
    x: float
    y: float
    size: float
    fill: str
    family: str
    anchor: str = "start"
    baseline: str = "alphabetic"


@dataclass
class AnchorPlacement:
    x: float
    y: float
    kind: str
    glyph_index: int
    align: str


@dataclass
class SheetPlacement:
    width: int
    height: int
    font_path: str
    resolved_family: str
    title: str
    texts: list[TextPlacement]
    glyph_runs: list[list[GlyphPlacement]]
    anchors: list[AnchorPlacement]


def run_fontconfig(query: str) -> tuple[str, str, str]:
    output = subprocess.check_output(
        ["fc-match", "-f", "%{file}\n%{family}\n%{style}\n", query],
        text=True,
        stderr=subprocess.DEVNULL,
    ).splitlines()
    if len(output) < 3:
        raise RuntimeError(f"fontconfig did not resolve {query!r}")
    return output[0], output[1], output[2]


def resolve_font(family: str, style: str) -> tuple[str, str, str]:
    query = family if not style else f"{family}:style={style}"
    path, resolved_family, resolved_style = run_fontconfig(query)
    if not os.path.exists(path):
        raise FileNotFoundError(path)
    return path, resolved_family, resolved_style


def load_font(path: str, size: float) -> ImageFont.FreeTypeFont:
    return ImageFont.truetype(path, max(1, int(round(size))))


def load_label_font(size: int) -> ImageFont.ImageFont:
    candidates = [
        "/usr/share/fonts/truetype/dejavu/DejaVuSans.ttf",
        "/usr/share/fonts/truetype/liberation2/LiberationSans-Regular.ttf",
    ]
    for path in candidates:
        if os.path.exists(path):
            return ImageFont.truetype(path, size)
    return ImageFont.load_default()


def parse_pattern(pattern: str) -> list[Token]:
    tokens: list[Token] = []
    pending_script = "normal"
    for index, char in enumerate(pattern):
        if char == "^":
            pending_script = "superscript"
            continue
        if char == "_":
            pending_script = "subscript"
            continue
        tokens.append(Token(char=char, script=pending_script))
        next_char = pattern[index + 1] if index + 1 < len(pattern) else ""
        if pending_script != "normal" and char.isdigit() and (next_char.isdigit() or next_char in "+-"):
            continue
        pending_script = "normal"
    return tokens


def split_layout_spec(pattern: str) -> tuple[str, int | None, str]:
    align = "right"
    align_marker = pattern.rfind("#")
    if align_marker > 0:
        suffix = pattern[align_marker + 1 :]
        if suffix in ALIGN_VALUES:
            align = suffix
            pattern = pattern[:align_marker]

    marker = pattern.rfind("@")
    if marker <= 0:
        return pattern, None, align
    suffix = pattern[marker + 1 :]
    if not suffix.isdigit():
        return pattern, None, align
    return pattern[:marker], int(suffix), align


def script_font_size(base_font_size: float, script: str) -> float:
    if script == "normal":
        return base_font_size
    return base_font_size * 0.78


def script_baseline_shift(base_font_size: float, script: str) -> float:
    if script == "superscript":
        return -base_font_size * 0.28
    if script == "subscript":
        return base_font_size * 0.30
    return 0.0


def shape_for_char(char: str) -> str:
    return "ellipse" if char in ELLIPSE_CHARS else "rect"


def normalize_bbox_bottom(char: str, bbox: tuple[float, float, float, float]) -> tuple[float, float, float, float]:
    if char.isalpha() and char not in DESCENDER_CHARS:
        return (bbox[0], bbox[1], bbox[2], min(bbox[3], 0.0))
    return bbox


def charge_sign_baseline_adjustment(font: ImageFont.FreeTypeFont, char: str, script: str) -> float:
    if script == "normal" or char not in "+-":
        return 0.0
    digit_bbox = font.getbbox("2", anchor="ls")
    sign_bbox = font.getbbox(char, anchor="ls")
    digit_center = (digit_bbox[1] + digit_bbox[3]) * 0.5
    sign_center = (sign_bbox[1] + sign_bbox[3]) * 0.5
    return digit_center - sign_center


def layout_tokens(
    font_path: str,
    tokens: list[Token],
    origin_x: float,
    baseline_y: float,
    base_font_size: float,
    pad_em: float,
) -> list[GlyphPlacement]:
    placements: list[GlyphPlacement] = []
    cursor_x = origin_x
    for token in tokens:
        size = script_font_size(base_font_size, token.script)
        font = load_font(font_path, size)
        advance = float(font.getlength(token.char))
        shifted_baseline = (
            baseline_y
            + script_baseline_shift(base_font_size, token.script)
            + charge_sign_baseline_adjustment(font, token.char, token.script)
        )
        raw_bbox = tuple(float(value) for value in font.getbbox(token.char, anchor="ls"))
        bbox = normalize_bbox_bottom(token.char, raw_bbox)
        pad = size * pad_em
        glyph_box = (
            cursor_x + bbox[0],
            shifted_baseline + bbox[1],
            cursor_x + bbox[2],
            shifted_baseline + bbox[3],
        )
        background = (
            glyph_box[0] - pad,
            glyph_box[1] - pad,
            glyph_box[2] + pad,
            glyph_box[3] + pad,
        )
        placements.append(
            GlyphPlacement(
                char=token.char,
                script=token.script,
                x=cursor_x,
                baseline_y=shifted_baseline,
                font_size=size,
                advance=advance,
                bbox=bbox,
                shape=shape_for_char(token.char),
                background=background,
            )
        )
        cursor_x += advance
    return placements


def translate_placement(placement: GlyphPlacement, dx: float, dy: float) -> GlyphPlacement:
    x1, y1, x2, y2 = placement.background
    return GlyphPlacement(
        char=placement.char,
        script=placement.script,
        x=placement.x + dx,
        baseline_y=placement.baseline_y + dy,
        font_size=placement.font_size,
        advance=placement.advance,
        bbox=placement.bbox,
        shape=placement.shape,
        background=(x1 + dx, y1 + dy, x2 + dx, y2 + dy),
    )


def resolve_anchor_index(tokens: list[Token], anchor_index: int | None) -> int | None:
    if anchor_index is not None and 0 <= anchor_index < len(tokens) and tokens[anchor_index].char != " ":
        return anchor_index
    for index, token in enumerate(tokens):
        if token.char != " ":
            return index
    return None


def layout_pattern(
    font_path: str,
    pattern: str,
    origin_x: float,
    baseline_y: float,
    base_font_size: float,
    pad_em: float,
    anchor_index: int | None = None,
    align: str = "right",
) -> list[GlyphPlacement]:
    tokens = parse_pattern(pattern)
    if not tokens:
        return []

    resolved_index = resolve_anchor_index(tokens, anchor_index)
    if resolved_index is None:
        return layout_tokens(font_path, tokens, origin_x, baseline_y, base_font_size, pad_em)

    if align in {"right", "left"}:
        placements = layout_tokens(font_path, tokens, 0.0, baseline_y, base_font_size, pad_em)
        dx = origin_x - placements[resolved_index].x
        return [translate_placement(placement, dx, 0.0) for placement in placements]

    placements: list[GlyphPlacement | None] = [None] * len(tokens)
    anchor = layout_tokens(font_path, [tokens[resolved_index]], origin_x, baseline_y, base_font_size, pad_em)[0]
    placements[resolved_index] = anchor

    other_indices = [index for index in range(len(tokens)) if index != resolved_index]
    other_tokens = [tokens[index] for index in other_indices]
    if not other_tokens:
        return [anchor]

    other = layout_tokens(font_path, other_tokens, origin_x, baseline_y, base_font_size, pad_em)
    dy = 0.0

    if align in {"above", "below"}:
        anchor_x1, anchor_y1, anchor_x2, anchor_y2 = placements_bounds([anchor])
        other_x1, other_y1, other_x2, other_y2 = placements_bounds(other)
        gap = base_font_size * 0.12
        if align == "above":
            dy = anchor_y1 - gap - other_y2
        else:
            dy = anchor_y2 + gap - other_y1

    for source_index, placement in zip(other_indices, other):
        placements[source_index] = translate_placement(placement, 0.0, dy)

    return [placement for placement in placements if placement is not None]


def locate_pattern(
    font_path: str,
    placements: list[GlyphPlacement],
    base_font_size: float,
    anchor_index: int | None,
    align: str,
) -> AnchorPlacement | None:
    if not placements:
        return None

    resolved_index = None
    if anchor_index is not None and 0 <= anchor_index < len(placements) and placements[anchor_index].char != " ":
        resolved_index = anchor_index
    if resolved_index is None:
        for index, placement in enumerate(placements):
            if placement.char != " ":
                resolved_index = index
                break
    if resolved_index is None:
        return None

    anchor_glyph = placements[resolved_index]
    standard_font = load_font(font_path, base_font_size)
    standard_bbox = normalize_bbox_bottom("H", tuple(float(value) for value in standard_font.getbbox("H", anchor="ls")))
    standard_center_offset = (standard_bbox[1] + standard_bbox[3]) * 0.5
    return AnchorPlacement(
        (anchor_glyph.background[0] + anchor_glyph.background[2]) * 0.5,
        anchor_glyph.baseline_y + standard_center_offset,
        "glyph-standard-center",
        resolved_index,
        align,
    )


def placements_bounds(placements: Iterable[GlyphPlacement]) -> tuple[float, float, float, float]:
    boxes = [placement.background for placement in placements]
    if not boxes:
        return 0.0, 0.0, 0.0, 0.0
    return (
        min(box[0] for box in boxes),
        min(box[1] for box in boxes),
        max(box[2] for box in boxes),
        max(box[3] for box in boxes),
    )


def draw_text_anchor_ls(
    image: Image.Image,
    font_path: str,
    text: str,
    font_size: float,
    x: float,
    baseline_y: float,
    fill: str,
) -> None:
    font = load_font(font_path, font_size)
    ImageDraw.Draw(image).text((x, baseline_y), text, font=font, fill=fill, anchor="ls")


def draw_placements(image: Image.Image, font_path: str, placements: list[GlyphPlacement]) -> None:
    draw = ImageDraw.Draw(image)
    for placement in placements:
        if placement.char == " ":
            continue
        x1, y1, x2, y2 = placement.background
        if placement.shape == "ellipse":
            draw.ellipse((x1, y1, x2, y2), fill="#ffffff")
        else:
            draw.rectangle((x1, y1, x2, y2), fill="#ffffff")
    for placement in placements:
        if placement.char == " ":
            continue
        draw_text_anchor_ls(
            image,
            font_path,
            placement.char,
            placement.font_size,
            placement.x,
            placement.baseline_y,
            "#050505",
        )


def draw_anchor(draw: ImageDraw.ImageDraw, anchor: AnchorPlacement) -> None:
    radius = 4.6
    draw.ellipse(
        (anchor.x - radius, anchor.y - radius, anchor.x + radius, anchor.y + radius),
        fill="#ffd400",
        outline="#050505",
        width=1,
    )


def sample_cell_height(font_path: str, patterns: Iterable[str], base_font_size: float, pad_em: float) -> float:
    max_height = 0.0
    for pattern in patterns:
        display_pattern, anchor_index, align = split_layout_spec(pattern)
        placements = layout_pattern(font_path, display_pattern, 0.0, 0.0, base_font_size, pad_em, anchor_index, align)
        _, y1, _, y2 = placements_bounds(placements)
        max_height = max(max_height, y2 - y1)
    return max(86.0, max_height + 58.0)


def count_section_rows(sample_count: int, columns: int) -> int:
    return int(math.ceil(sample_count / columns))


def draw_label(draw: ImageDraw.ImageDraw, xy: tuple[float, float], text: str, font: ImageFont.ImageFont, fill: str) -> None:
    draw.text(xy, text, font=font, fill=fill, anchor="la")


def build_sheet(
    font_path: str,
    resolved_family: str,
    title: str,
    base_font_size: float,
    pad_em: float,
    columns: int,
    width: int,
) -> SheetPlacement:
    margin_x = 56
    margin_y = 46
    column_gap = 34
    section_gap = 40
    header_gap = 98
    section_title_height = 28
    cell_width = (width - margin_x * 2 - column_gap * (columns - 1)) / columns

    cell_height = sample_cell_height(
        font_path,
        (pattern for _, patterns in SHEET_SECTIONS for pattern in patterns),
        base_font_size,
        pad_em,
    )
    total_rows = sum(count_section_rows(len(patterns), columns) for _, patterns in SHEET_SECTIONS)
    height = int(margin_y * 2 + header_gap + total_rows * cell_height + len(SHEET_SECTIONS) * (section_title_height + section_gap))

    resolved = os.path.realpath(font_path)
    texts = [
        TextPlacement(title, margin_x, margin_y, 34, "#f3f3f3", "DejaVu Sans", baseline="hanging"),
        TextPlacement(f"font file: {resolved}", margin_x, margin_y + 43, 16, "#a8a8a8", "DejaVu Sans", baseline="hanging"),
        TextPlacement(
            f"base size={base_font_size:.0f}px, script scale=0.78, pad={pad_em:.2f}em, ellipse glyphs={''.join(sorted(ELLIPSE_CHARS))}",
            margin_x,
            margin_y + 67,
            16,
            "#a8a8a8",
            "DejaVu Sans",
            baseline="hanging",
        ),
    ]
    glyph_runs: list[list[GlyphPlacement]] = []
    anchors: list[AnchorPlacement] = []

    y = margin_y + header_gap
    for section_title, patterns in SHEET_SECTIONS:
        texts.append(TextPlacement(section_title, margin_x, y, 22, "#f0f0f0", "DejaVu Sans", baseline="hanging"))
        y += section_title_height
        for index, pattern in enumerate(patterns):
            display_pattern, anchor_index, align = split_layout_spec(pattern)
            col = index % columns
            row = index // columns
            x = margin_x + col * (cell_width + column_gap)
            cell_y = y + row * cell_height
            texts.append(TextPlacement(pattern, x, cell_y, 15, "#9d9d9d", "DejaVu Sans", baseline="hanging"))
            baseline_y = cell_y + 78
            layout_x = x + (70.0 if align == "left" else 0.0)
            placements = layout_pattern(font_path, display_pattern, layout_x, baseline_y, base_font_size, pad_em, anchor_index, align)
            glyph_runs.append(placements)
            anchor = locate_pattern(font_path, placements, base_font_size, anchor_index, align)
            if anchor:
                anchors.append(anchor)
        y += count_section_rows(len(patterns), columns) * cell_height + section_gap

    return SheetPlacement(
        width=width,
        height=height,
        font_path=font_path,
        resolved_family=resolved_family,
        title=title,
        texts=texts,
        glyph_runs=glyph_runs,
        anchors=anchors,
    )


def render_png(sheet: SheetPlacement, output_path: str) -> None:
    image = Image.new("RGB", (sheet.width, sheet.height), "#050505")
    draw = ImageDraw.Draw(image)
    label_fonts: dict[int, ImageFont.ImageFont] = {}
    for text in sheet.texts:
        size = int(round(text.size))
        if size not in label_fonts:
            label_fonts[size] = load_label_font(size)
        anchor = "la" if text.baseline == "hanging" else "ls"
        draw.text((text.x, text.y), text.text, font=label_fonts[size], fill=text.fill, anchor=anchor)
    for placements in sheet.glyph_runs:
        draw_placements(image, sheet.font_path, placements)
    for anchor in sheet.anchors:
        draw_anchor(draw, anchor)
    os.makedirs(os.path.dirname(output_path) or ".", exist_ok=True)
    image.save(output_path)


def render_svg(sheet: SheetPlacement, output_path: str) -> None:
    lines = [
        f'<svg xmlns="http://www.w3.org/2000/svg" width="{sheet.width}" height="{sheet.height}" viewBox="0 0 {sheet.width} {sheet.height}">',
        '  <rect width="100%" height="100%" fill="#050505"/>',
    ]
    for text in sheet.texts:
        lines.append(
            f'  <text x="{text.x:.3f}" y="{text.y:.3f}" fill="{text.fill}" font-size="{text.size:.3f}" '
            f'font-family="{html.escape(text.family)}" dominant-baseline="{text.baseline}" text-anchor="{text.anchor}">'
            f'{html.escape(text.text)}</text>'
        )
    for placements in sheet.glyph_runs:
        for placement in placements:
            if placement.char == " ":
                continue
            x1, y1, x2, y2 = placement.background
            if placement.shape == "ellipse":
                lines.append(
                    f'  <ellipse cx="{(x1 + x2) * 0.5:.3f}" cy="{(y1 + y2) * 0.5:.3f}" '
                    f'rx="{(x2 - x1) * 0.5:.3f}" ry="{(y2 - y1) * 0.5:.3f}" fill="#ffffff"/>'
                )
            else:
                lines.append(
                    f'  <rect x="{x1:.3f}" y="{y1:.3f}" width="{x2 - x1:.3f}" height="{y2 - y1:.3f}" fill="#ffffff"/>'
                )
    for placements in sheet.glyph_runs:
        for placement in placements:
            if placement.char == " ":
                continue
            lines.append(
                f'  <text x="{placement.x:.3f}" y="{placement.baseline_y:.3f}" fill="#050505" '
                f'font-size="{placement.font_size:.3f}" font-family="{html.escape(sheet.resolved_family)}" '
                f'dominant-baseline="alphabetic">{html.escape(placement.char)}</text>'
            )
    for anchor in sheet.anchors:
        lines.append(
            f'  <circle cx="{anchor.x:.3f}" cy="{anchor.y:.3f}" r="4.600" fill="#ffd400" '
            f'stroke="#050505" stroke-width="1.000" data-anchor-kind="{anchor.kind}" '
            f'data-anchor-index="{anchor.glyph_index}" data-align="{anchor.align}"/>'
        )
    lines.append("</svg>")
    os.makedirs(os.path.dirname(output_path) or ".", exist_ok=True)
    with open(output_path, "w", encoding="utf-8") as file:
        file.write("\n".join(lines))


def main() -> int:
    parser = argparse.ArgumentParser(description="Render detailed per-font glyph background preview sheets.")
    parser.add_argument("--family", required=True, help="Font family query, e.g. Arial or Times New Roman.")
    parser.add_argument("--style", default="Regular", help="Font style query, e.g. Regular or Italic.")
    parser.add_argument("--output", required=True)
    parser.add_argument("--svg-output", default=None)
    parser.add_argument("--title", default=None)
    parser.add_argument("--font-size", type=float, default=36.0)
    parser.add_argument("--pad-em", type=float, default=0.09)
    parser.add_argument("--columns", type=int, default=4)
    parser.add_argument("--width", type=int, default=1600)
    args = parser.parse_args()

    font_path, resolved_family, resolved_style = resolve_font(args.family, args.style)
    title = args.title or f"{args.family} {args.style} preview ({resolved_family} {resolved_style})"
    sheet = build_sheet(
        font_path=font_path,
        resolved_family=resolved_family,
        title=title,
        base_font_size=args.font_size,
        pad_em=args.pad_em,
        columns=args.columns,
        width=args.width,
    )
    render_png(sheet, args.output)
    svg_output = args.svg_output
    if svg_output is None:
        root, _ = os.path.splitext(args.output)
        svg_output = f"{root}.svg"
    render_svg(sheet, svg_output)
    print(f"wrote {args.output}")
    print(f"wrote {svg_output}")
    print(f"resolved font: {font_path} ({resolved_family} {resolved_style})")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
