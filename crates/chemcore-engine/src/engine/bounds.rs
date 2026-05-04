use super::RenderBoundsScope;
use crate::{RenderPrimitive, RenderRole};

pub(super) fn render_bounds_scope_accepts(
    scope: RenderBoundsScope,
    primitive: &RenderPrimitive,
) -> bool {
    match scope {
        RenderBoundsScope::All => true,
        RenderBoundsScope::Document => {
            let role = render_primitive_role(primitive);
            role != RenderRole::DocumentKnockout
                && !render_role_is_selection(role)
                && !render_role_is_hover(role)
                && !render_role_is_preview(role)
        }
        RenderBoundsScope::Selection => render_role_is_selection(render_primitive_role(primitive)),
    }
}

fn render_primitive_role(primitive: &RenderPrimitive) -> RenderRole {
    match primitive {
        RenderPrimitive::Line { role, .. }
        | RenderPrimitive::Circle { role, .. }
        | RenderPrimitive::Polygon { role, .. }
        | RenderPrimitive::Rect { role, .. }
        | RenderPrimitive::Ellipse { role, .. }
        | RenderPrimitive::Polyline { role, .. }
        | RenderPrimitive::Path { role, .. }
        | RenderPrimitive::FilledPath { role, .. }
        | RenderPrimitive::Text { role, .. } => *role,
    }
}

fn render_role_is_selection(role: RenderRole) -> bool {
    matches!(
        role,
        RenderRole::SelectionBox
            | RenderRole::SelectionBond
            | RenderRole::SelectionBondDot
            | RenderRole::SelectionNode
            | RenderRole::SelectionTextBox
    )
}

fn render_role_is_hover(role: RenderRole) -> bool {
    matches!(
        role,
        RenderRole::HoverEndpoint
            | RenderRole::HoverLabelGlyph
            | RenderRole::HoverBondCenter
            | RenderRole::HoverArrowCenter
            | RenderRole::HoverArrowHandle
            | RenderRole::HoverTextBox
    )
}

fn render_role_is_preview(role: RenderRole) -> bool {
    matches!(role, RenderRole::PreviewBond | RenderRole::PreviewEnd)
}
