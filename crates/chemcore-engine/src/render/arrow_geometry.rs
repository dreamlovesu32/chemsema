use super::*;

pub(super) fn arrow_head_points(
    from: Point,
    to: Point,
    arrow_head: ArrowHeadGeometry,
) -> Vec<Point> {
    let direction = Vector::new(to.x - from.x, to.y - from.y);
    let unit = direction.normalized();
    let normal = Vector::new(-unit.y, unit.x);
    let head_length = arrow_head
        .length
        .max(crate::ARROW_SHAPE_MIN_HEAD_LENGTH_CM.value());
    let head_half_width =
        (arrow_head.width.max(0.0) + 0.05).max(crate::ARROW_SHAPE_MIN_HEAD_WIDTH_CM.value());
    let notch_length = arrow_head
        .center_length
        .max(crate::ARROW_SHAPE_MIN_NOTCH_LENGTH_CM.value())
        .min(head_length - crate::ARROW_SHAPE_MIN_HEAD_TO_NOTCH_GAP_CM.value());
    let tip = to;
    let left = Point::new(
        to.x - unit.x * head_length + normal.x * head_half_width,
        to.y - unit.y * head_length + normal.y * head_half_width,
    );
    let right = Point::new(
        to.x - unit.x * head_length - normal.x * head_half_width,
        to.y - unit.y * head_length - normal.y * head_half_width,
    );
    if arrow_head.head_full && notch_length < head_length - 0.2 {
        let notch = Point::new(to.x - unit.x * notch_length, to.y - unit.y * notch_length);
        vec![tip, left, notch, right]
    } else {
        vec![tip, left, right]
    }
}

pub(super) fn arrow_axis(from: Point, to: Point) -> Option<(Vector, Vector, f64)> {
    let direction = Vector::new(to.x - from.x, to.y - from.y);
    let length = direction.length();
    if length <= EPSILON {
        return None;
    }
    let unit = direction.normalized();
    let normal = Vector::new(-unit.y, unit.x);
    Some((unit, normal, length))
}

pub(super) fn hollow_arrow_outline_points(
    start: Point,
    end: Point,
    arrow_head: ArrowHeadGeometry,
    has_head: bool,
    has_tail: bool,
) -> Option<Vec<Point>> {
    let (unit, normal, length) = arrow_axis(start, end)?;
    let shaft_half_width = arrow_head.center_length.max(arrow_head.length) * 0.5;
    let head_length = arrow_head.length.min(length * 0.45);
    let head_half_width = shaft_half_width + arrow_head.width.max(0.0) * 0.5;
    let neck_offset = (head_length * 0.5).min(length * 0.3);
    let start_neck = if has_tail {
        start.translated(unit.scaled(neck_offset))
    } else {
        start
    };
    let end_neck = if has_head {
        end.translated(unit.scaled(-neck_offset))
    } else {
        end
    };

    if !has_head && !has_tail {
        return Some(vec![
            start.translated(normal.scaled(shaft_half_width)),
            end.translated(normal.scaled(shaft_half_width)),
            end.translated(normal.scaled(-shaft_half_width)),
            start.translated(normal.scaled(-shaft_half_width)),
        ]);
    }

    let mut points = Vec::new();
    if has_tail {
        let tail_outer = start.translated(unit.scaled(head_length));
        points.push(start);
        points.push(tail_outer.translated(normal.scaled(-head_half_width)));
    } else {
        points.push(start.translated(normal.scaled(-shaft_half_width)));
    }
    points.push(start_neck.translated(normal.scaled(-shaft_half_width)));
    points.push(end_neck.translated(normal.scaled(-shaft_half_width)));
    if has_head {
        let head_outer = end.translated(unit.scaled(-head_length));
        points.push(head_outer.translated(normal.scaled(-head_half_width)));
        points.push(end);
        points.push(head_outer.translated(normal.scaled(head_half_width)));
    } else {
        points.push(end.translated(normal.scaled(-shaft_half_width)));
        points.push(end.translated(normal.scaled(shaft_half_width)));
    }
    points.push(end_neck.translated(normal.scaled(shaft_half_width)));
    points.push(start_neck.translated(normal.scaled(shaft_half_width)));
    if has_tail {
        let tail_outer = start.translated(unit.scaled(head_length));
        points.push(tail_outer.translated(normal.scaled(head_half_width)));
    } else {
        points.push(start.translated(normal.scaled(shaft_half_width)));
    }
    Some(compact_polygon_points(points))
}
