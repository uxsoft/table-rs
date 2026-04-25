use iced::widget::{column as iced_column, container, row as iced_row, text};
use iced::{Background, Border, Element, Length, Padding, Shadow};
use iced_longbridge::components::button::{button_ex, Variant};
use iced_longbridge::components::hover_card::hover_card;
use iced_longbridge::components::input::input_sized;
use iced_longbridge::components::popover::popover_dismissable;
use iced_longbridge::components::tooltip::wrap as tooltip_wrap;
use iced_longbridge::theme::Size;

use crate::ui::icons::{icon, icon_button, icon_colored, IconKind};
use crate::{Message, TableApp};

const SYNTAX_HELP: &str = "Formula syntax\n\n\
    {ColumnName}   value of that column in this row\n\
    + - * /        arithmetic\n\
    ( )            grouping\n\
\n\
Examples\n\
    {Price} * {Quantity}\n\
    ({Value} + 10) / 2";

pub fn view(app: &TableApp) -> Option<Element<'_, Message>> {
    let col = app.formula.editing_col?;
    let theme = &app.theme;
    let t = *theme;

    let col_name = app
        .sheet
        .columns
        .get(col)
        .map(|c| c.name.clone())
        .unwrap_or_else(|| format!("col_{col}"));

    let fx_trigger: Element<'_, Message> = container(icon_colored(
        IconKind::FunctionSquare,
        16.0,
        theme.muted_foreground,
    ))
    .width(Length::Fixed(22.0))
    .align_x(iced::alignment::Horizontal::Center)
    .into();
    let fx_help: Element<'_, Message> = text(SYNTAX_HELP)
        .size(12.0)
        .color(theme.popover_foreground)
        .into();
    let fx_label = hover_card(theme, fx_trigger, fx_help);

    let col_label = text(col_name)
        .size(13.0)
        .color(theme.foreground)
        .width(Length::Fixed(120.0));

    let input: Element<'_, Message> =
        input_sized(theme, Size::Sm, "= expression…", &app.formula.value)
            .on_input(Message::FormulaChanged)
            .on_submit(Message::FormulaEditCommit)
            .width(Length::Fill)
            .into();

    let suggestions = app.formula.suggestions(&app.sheet);
    let suggestion_panel: Option<Element<'_, Message>> = if suggestions.is_empty() {
        None
    } else {
        let selected = app.formula.suggestions_selected.min(suggestions.len() - 1);
        let rows: Vec<Element<'_, Message>> = suggestions
            .iter()
            .enumerate()
            .map(|(i, (_col_idx, name))| {
                let variant = if i == selected {
                    Variant::Secondary
                } else {
                    Variant::Ghost
                };
                button_ex(
                    theme,
                    name.clone(),
                    variant,
                    Size::Sm,
                    Some(Message::FormulaSuggestionClick(i)),
                    false,
                    false,
                )
            })
            .collect();
        Some(
            iced_column(rows)
                .spacing(2)
                .width(Length::Fixed(220.0))
                .into(),
        )
    };

    let input_with_popover =
        popover_dismissable(theme, input, suggestion_panel, Message::FormulaEscape);

    let error_slot: Element<'_, Message> = if let Some(err) = &app.formula.error {
        let dot: Element<'_, Message> = container(text(""))
            .width(Length::Fixed(8.0))
            .height(Length::Fixed(8.0))
            .style(move |_| container::Style {
                background: Some(Background::Color(t.danger)),
                text_color: Some(t.danger_foreground),
                border: Border {
                    color: t.danger,
                    width: 0.0,
                    radius: 4.0.into(),
                },
                shadow: Shadow::default(),
                snap: true,
            })
            .into();
        let wrapper: Element<'_, Message> = container(dot)
            .width(Length::Fixed(14.0))
            .height(Length::Fixed(22.0))
            .padding(Padding::from([7.0, 3.0]))
            .into();
        tooltip_wrap(theme, wrapper, format!("Formula didn't evaluate: {err}")).into()
    } else {
        container(text(""))
            .width(Length::Fixed(14.0))
            .height(Length::Fixed(22.0))
            .into()
    };

    let commit = icon_button(
        theme,
        icon_colored(IconKind::Check, 14.0, theme.primary_foreground),
        Variant::Primary,
        Size::Sm,
        Some(Message::FormulaEditCommit),
        false,
    );

    let cancel = icon_button(
        theme,
        icon(theme, IconKind::Close, 14.0),
        Variant::Ghost,
        Size::Sm,
        Some(Message::FormulaEditCancel),
        false,
    );

    let bar = iced_row![fx_label, col_label, input_with_popover, error_slot, commit, cancel]
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
