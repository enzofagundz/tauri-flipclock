//! Thin adapter over `gtk-layer-shell` so the windowing backend can be replaced.

use flipclock_core::settings::Corner;
use gtk::prelude::{GtkWindowExt, WidgetExt};
use gtk_layer_shell::{Edge, KeyboardMode, Layer, LayerShell};

/// Gap between the pinned surface and the screen edges, in logical pixels.
pub const MARGIN: i32 = 24;

pub fn is_supported() -> bool {
    gtk_layer_shell::is_supported()
}

/// Turns `window` into a layer surface on the overlay layer.
///
/// Must run on the GTK main thread and before the window is realized. Returns
/// `false` when GTK refused to make the window a layer surface.
pub fn setup(window: &gtk::ApplicationWindow) -> bool {
    window.init_layer_shell();
    if !window.is_layer_window() {
        return false;
    }
    window.set_namespace("flipclock");
    window.set_layer(Layer::Overlay);
    window.set_keyboard_mode(KeyboardMode::OnDemand);
    window.set_exclusive_zone(0);
    true
}

/// Applies anchors and forces the surface to recompute its size.
///
/// gtk-layer-shell ignores the size request when the window is anchored to
/// opposite edges, and a bare anchor change does not resize an exported
/// surface; `resize(1, 1)` forces that recomputation. `size` is `None` in
/// fullscreen, where the compositor owns the size.
pub fn apply_geometry(
    window: &gtk::ApplicationWindow,
    corner: Corner,
    fullscreen: bool,
    size: Option<(i32, i32)>,
) {
    apply_anchors(window, corner, fullscreen);
    match size {
        Some((width, height)) => window.set_size_request(width, height),
        None => window.set_size_request(-1, -1),
    }
    window.resize(1, 1);
}

/// Anchors the surface to `corner` with a [`MARGIN`] gap, or to every edge for
/// fullscreen.
pub fn apply_anchors(window: &gtk::ApplicationWindow, corner: Corner, fullscreen: bool) {
    let (vertical, horizontal) = corner_edges(corner);

    if fullscreen {
        for edge in [Edge::Top, Edge::Bottom, Edge::Left, Edge::Right] {
            window.set_anchor(edge, true);
            window.set_layer_shell_margin(edge, 0);
        }
        return;
    }

    window.set_anchor(Edge::Top, vertical == Edge::Top);
    window.set_anchor(Edge::Bottom, vertical == Edge::Bottom);
    window.set_anchor(Edge::Left, horizontal == Edge::Left);
    window.set_anchor(Edge::Right, horizontal == Edge::Right);

    window.set_layer_shell_margin(Edge::Top, 0);
    window.set_layer_shell_margin(Edge::Bottom, 0);
    window.set_layer_shell_margin(Edge::Left, 0);
    window.set_layer_shell_margin(Edge::Right, 0);
    window.set_layer_shell_margin(vertical, MARGIN);
    window.set_layer_shell_margin(horizontal, MARGIN);
}

fn corner_edges(corner: Corner) -> (Edge, Edge) {
    match corner {
        Corner::TopLeft => (Edge::Top, Edge::Left),
        Corner::TopRight => (Edge::Top, Edge::Right),
        Corner::BottomLeft => (Edge::Bottom, Edge::Left),
        Corner::BottomRight => (Edge::Bottom, Edge::Right),
    }
}

#[cfg(test)]
mod tests {
    use super::corner_edges;
    use flipclock_core::settings::Corner;
    use gtk_layer_shell::Edge;

    #[test]
    fn every_corner_uses_one_vertical_and_one_horizontal_edge() {
        assert_eq!(corner_edges(Corner::TopLeft), (Edge::Top, Edge::Left));
        assert_eq!(corner_edges(Corner::TopRight), (Edge::Top, Edge::Right));
        assert_eq!(corner_edges(Corner::BottomLeft), (Edge::Bottom, Edge::Left));
        assert_eq!(
            corner_edges(Corner::BottomRight),
            (Edge::Bottom, Edge::Right)
        );
    }
}
