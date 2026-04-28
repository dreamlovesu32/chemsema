use crate::{css_px, world_cm, WorldCm, DEFAULT_BOND_STROKE_CM};

pub const VIEWER_BOND_STROKE_CM: WorldCm = world_cm(DEFAULT_BOND_STROKE_CM);

pub const DOUBLE_BOND_SIDE_INSET_CM: WorldCm = css_px(1.4).to_world_cm();
pub const HASH_WEDGE_SPACING_CM: WorldCm = css_px(3.2).to_world_cm();
pub const HASH_WEDGE_START_OFFSET_CM: WorldCm = css_px(1.95).to_world_cm();
pub const HASH_WEDGE_END_INSET_CM: WorldCm = css_px(0.18).to_world_cm();
pub const HASH_BLACK_SEGMENT_LENGTH_CM: WorldCm = css_px(0.5).to_world_cm();
pub const HASH_TARGET_GAP_LENGTH_CM: WorldCm = css_px(0.65).to_world_cm();
pub const HASH_WEDGE_EDGE_OVERDRAW_CM: WorldCm = css_px(0.28).to_world_cm();
pub const HASH_MULTI_BOND_RETREAT_GAP_CM: WorldCm = css_px(0.45).to_world_cm();
pub const SOLID_WEDGE_END_INSET_CM: WorldCm = css_px(0.55).to_world_cm();
pub const BOLD_BOND_WIDTH_CM: WorldCm = world_cm(0.141);
pub const SOLID_WEDGE_HALF_WIDTH_CM: WorldCm = world_cm(0.10575);
pub const SOLID_WEDGE_TIP_HALF_WIDTH_CM: WorldCm = world_cm(0.0175);
pub const DASHED_BOND_PATTERN_CM: [WorldCm; 2] =
    [css_px(3.2).to_world_cm(), css_px(2.4).to_world_cm()];

pub const HASH_WEDGE_INITIAL_HALF_WIDTH_CM: WorldCm = css_px(0.42).to_world_cm();
pub const HASH_WEDGE_PROGRESS_BASE_HALF_WIDTH_CM: WorldCm = css_px(0.16).to_world_cm();
pub const HASH_WEDGE_PROGRESS_HALF_WIDTH_RANGE_CM: WorldCm = css_px(1.72).to_world_cm();
pub const HASH_WEDGE_INITIAL_SEGMENT_WIDTH_CM: WorldCm = css_px(0.82).to_world_cm();
pub const HASH_WEDGE_SEGMENT_WIDTH_CM: WorldCm = css_px(0.72).to_world_cm();

pub const BOUNDARY_JOIN_MIN_BACKTRACK_CM: WorldCm = css_px(0.85).to_world_cm();
pub const LABEL_GEOMETRY_CLIP_MARGIN_CM: WorldCm = css_px(1.8).to_world_cm();
pub const DEFAULT_ARROW_HEAD_LENGTH_CM: WorldCm = css_px(8.0).to_world_cm();
pub const ARROW_SHAPE_MIN_HEAD_LENGTH_CM: WorldCm = css_px(5.4).to_world_cm();
pub const ARROW_SHAPE_MIN_HEAD_WIDTH_CM: WorldCm = css_px(4.8).to_world_cm();
pub const ARROW_SHAPE_MIN_NOTCH_LENGTH_CM: WorldCm = css_px(3.2).to_world_cm();
pub const ARROW_SHAPE_MIN_HEAD_TO_NOTCH_GAP_CM: WorldCm = css_px(0.8).to_world_cm();
pub const TEXT_WRAP_ESTIMATED_CHAR_WIDTH_CM: WorldCm = css_px(6.0).to_world_cm();
pub const DASH_GAP_STROKE_EXTRA_CM: WorldCm = css_px(0.35).to_world_cm();
pub const HASH_WEDGE_GAP_START_OFFSET_CM: WorldCm = css_px(0.5).to_world_cm();
pub const HASH_WEDGE_GAP_END_INSET_CM: WorldCm = css_px(0.18).to_world_cm();
