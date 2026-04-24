use iced::widget::{container, row as iced_row, text};
use iced::{Background, Border, Element, Length, Padding, Shadow};
use iced_longbridge::components::button::{button_ex, Variant};
use iced_longbridge::components::input::input_sized;
use iced_longbridge::theme::Size;

use crate::{Message, TableApp};

pub fn view(app: &TableApp) -> Option<Element<'_, Message>> {
    let col = app.editing_formula_col?;
    let theme = &app.theme;
    let t = *theme;

    let col_name = app
        .sheet
        .columns
        .get(col)
        .map(|c| c.name.clone())
        .unwrap_or_else(|| format!("col_{col}"));

    let fx_label = text("fx")
        .size(13.0)
        .color(theme.muted_foreground)
        .width(Length::Fixed(22.0));

    let col_label = text(col_name)
        .size(13.0)
        .color(theme.foreground)
        .width(Length::Fixed(120.0));

    let input = input_sized(theme, Size::Sm, "= expression…", &app.editing_formula_value)
        .on_input(Message::FormulaChanged)
        .on_submit(Message::FormulaEditCommit)
        .width(Length::Fill);

    let commit = button_ex(
        theme,
        "Apply",
        Variant::Primary,
        Size::Sm,
        Some(Message::FormulaEditCommit),
        false,
        false,
    );

    let cancel = button_ex(
        theme,
        "Cancel",
        Variant::Ghost,
        Size::Sm,
        Some(Message::FormulaEditCancel),
        false,
        false,
    );

    let bar = iced_row![fx_label, col_label, input, commit, cancel]
        .spacing(8)
        .align_y(iced::alignment::Vertical::Center);

    Some(
        container(bar)
            .padding(Padding::from([6.0, 12.0]))
            .width(Length::Fill)
            .style(move |_| container::Style {
                background: Some(Background::Color(t.muted)),
                text_color: Some(t.foreground),
                border: Border {
                    color: t.border,
                    width: 0.0,
                    radius: 0.0.into(),
                },
                shadow: Shadow::default(),
                snap: true,
            })
            .into(),
    )
}
