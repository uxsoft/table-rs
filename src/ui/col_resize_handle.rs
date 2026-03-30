use iced::widget::{container, horizontal_space, mouse_area, row};
use iced::{Border, Element, Length};

use crate::Message;

const HANDLE_W: f32 = 8.0;

/// A resize drag handle overlaid on the right edge of a column header.
/// It is stacked on top of the header button so it captures press events
/// on the last `HANDLE_W` pixels without adding extra width to the column.
pub fn view<'a>(col_idx: usize, col_width: f32, header_height: f32, active: bool) -> Element<'a, Message> {
    let handle_bar = container(horizontal_space())
        .width(Length::Fixed(HANDLE_W))
        .height(Length::Fixed(header_height))
        .style(if active { active_style } else { idle_style });

    let handle = mouse_area(handle_bar)
        .on_press(Message::ColResizeStart(col_idx))
        .interaction(iced::mouse::Interaction::ResizingHorizontally);

    // horizontal_space() fills the column width; the bar sits flush at the right edge.
    row![horizontal_space(), handle]
        .width(Length::Fixed(col_width))
        .height(Length::Fixed(header_height))
        .into()
}

fn idle_style(theme: &iced::Theme) -> container::Style {
    let p = theme.extended_palette();
    // Slightly darker than the header background so the handle is subtly visible.
    container::Style {
        background: Some(p.primary.strong.color.into()),
        border: Border {
            color: p.primary.strong.color,
            width: 0.0,
            radius: 0.0.into(),
        },
        ..Default::default()
    }
}

fn active_style(theme: &iced::Theme) -> container::Style {
    let p = theme.extended_palette();
    container::Style {
        background: Some(p.primary.base.color.into()),
        border: Border {
            color: p.primary.base.color,
            width: 0.0,
            radius: 0.0.into(),
        },
        ..Default::default()
    }
}
