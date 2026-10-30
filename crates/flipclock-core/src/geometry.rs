use crate::settings::Scale;

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Size {
    pub width: f64,
    pub height: f64,
}

const PADDING: f64 = 16.0;
const GAP: f64 = 8.0;
const COLON_WIDTH: f64 = 16.0;
const DATE_HEIGHT: f64 = 28.0;
const DATE_GAP: f64 = 8.0;
const CARD_COUNT: f64 = 4.0;

/// Main card and smaller seconds card dimensions per scale preset.
fn card_size(scale: Scale) -> Size {
    match scale {
        Scale::Small => Size { width: 64.0, height: 92.0 },
        Scale::Medium => Size { width: 96.0, height: 136.0 },
        Scale::Large => Size { width: 144.0, height: 204.0 },
    }
}

fn seconds_card_size(scale: Scale) -> Size {
    match scale {
        Scale::Small => Size { width: 44.0, height: 64.0 },
        Scale::Medium => Size { width: 64.0, height: 92.0 },
        Scale::Large => Size { width: 96.0, height: 136.0 },
    }
}

/// Window size that fits `HH:MM` plus the enabled optional fields.
///
/// The card row is five blocks (`H H : M M`) separated by four gaps; the
/// seconds group appends `: S S` with three more gaps.
pub fn window_size(scale: Scale, show_seconds: bool, show_date: bool) -> Size {
    let card = card_size(scale);
    let mut width = PADDING * 2.0 + CARD_COUNT * card.width + GAP * 4.0 + COLON_WIDTH;

    if show_seconds {
        let seconds = seconds_card_size(scale);
        width += GAP + COLON_WIDTH + GAP + seconds.width + GAP + seconds.width;
    }

    let mut height = PADDING * 2.0 + card.height;
    if show_date {
        height += DATE_GAP + DATE_HEIGHT;
    }

    Size { width, height }
}
