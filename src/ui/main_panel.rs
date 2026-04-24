use iced::widget::column;
use iced::{Element, Length};

use crate::{ui, Message, TableApp};

pub fn view(app: &TableApp) -> Element<'_, Message> {
    let mut stack = column![].width(Length::Fill).height(Length::Fill).spacing(0);

    if let Some(formula) = ui::formula_bar::view(app) {
        stack = stack.push(formula);
    }

    stack = stack.push(ui::table_view::view(app));

    stack.into()
}
