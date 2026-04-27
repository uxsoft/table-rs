use iced::widget::column;
use iced::{Element, Length};

use crate::{ui, Message, TableApp};

pub fn view(app: &TableApp) -> Element<'_, Message> {
    let mut stack = column![].width(Length::Fill).height(Length::Fill).spacing(0);

    // Hide the standalone formula bar while the column-settings dropdown is
    // open for that column — the dropdown hosts the formula editor itself, so
    // showing both would let two inputs fight over the same state.
    let suppress_bar = app.column_settings_open.is_some()
        && app.column_settings_open == app.formula.editing_col;
    if !suppress_bar {
        if let Some(formula) = ui::formula_bar::view(app) {
            stack = stack.push(formula);
        }
    }

    stack = stack.push(ui::table_view::view(app));

    stack.into()
}
