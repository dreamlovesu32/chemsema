use super::*;

pub(super) fn colorref_from_css(value: &str) -> Option<COLORREF> {
    let value = value.trim();
    if let Some(hex) = value.strip_prefix('#') {
        if hex.len() != 6 {
            return None;
        }
        let rgb = u32::from_str_radix(hex, 16).ok()?;
        let r = (rgb >> 16) & 0xff;
        let g = (rgb >> 8) & 0xff;
        let b = rgb & 0xff;
        return Some((b << 16) | (g << 8) | r);
    }
    if let Some((r, g, b, alpha)) = parse_css_rgba(value) {
        if alpha <= 0.0 {
            return None;
        }
        let r = composite_css_channel_on_white(r, alpha);
        let g = composite_css_channel_on_white(g, alpha);
        let b = composite_css_channel_on_white(b, alpha);
        return Some((b << 16) | (g << 8) | r);
    }
    None
}

pub(super) fn parse_css_rgba(value: &str) -> Option<(u32, u32, u32, f64)> {
    let inner = value
        .strip_prefix("rgba(")
        .and_then(|rest| rest.strip_suffix(')'))
        .or_else(|| {
            value
                .strip_prefix("rgb(")
                .and_then(|rest| rest.strip_suffix(')'))
        })?;
    let parts: Vec<&str> = inner
        .split(',')
        .map(str::trim)
        .filter(|part| !part.is_empty())
        .collect();
    if parts.len() != 3 && parts.len() != 4 {
        return None;
    }
    let r = parse_css_channel(parts[0])?;
    let g = parse_css_channel(parts[1])?;
    let b = parse_css_channel(parts[2])?;
    let alpha = if parts.len() == 4 {
        parts[3].parse::<f64>().ok()?.clamp(0.0, 1.0)
    } else {
        1.0
    };
    Some((r, g, b, alpha))
}

pub(super) fn parse_css_channel(value: &str) -> Option<u32> {
    if let Some(percent) = value.strip_suffix('%') {
        let percent = percent.parse::<f64>().ok()?.clamp(0.0, 100.0);
        Some((percent * 255.0 / 100.0).round() as u32)
    } else {
        let channel = value.parse::<f64>().ok()?.clamp(0.0, 255.0);
        Some(channel.round() as u32)
    }
}

pub(super) fn composite_css_channel_on_white(channel: u32, alpha: f64) -> u32 {
    ((channel as f64 * alpha) + 255.0 * (1.0 - alpha))
        .round()
        .clamp(0.0, 255.0) as u32
}
