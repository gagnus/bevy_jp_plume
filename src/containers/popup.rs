//! Shared popup plumbing for controls that float a panel over the UI.
use bevy::picking::Pickable;
use bevy::scene::{Scene, bsn};
use bevy::ui::{Node, Overflow, PositionType, Val};

// Mount point for a control's popup: overlays the control exactly, so the popup's
// `Popover` anchors to the same rect. Clipped because taffy counts absolute
// children in `content_size` even while `Visibility::Hidden` — an unclipped popup
// inflates every ancestor scroll range (the popup itself escapes via `OverrideClip`).
pub(crate) fn popup_socket() -> impl Scene {
    bsn! {
        Node {
            position_type: PositionType::Absolute,
            left: Val::ZERO,
            right: Val::ZERO,
            top: Val::ZERO,
            bottom: Val::ZERO,
            overflow: Overflow::clip(),
        }
        Pickable::IGNORE
    }
}
