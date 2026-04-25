use std::time::Duration;

use iced_longbridge::components::notification::{
    Notification, NotificationKind, NotificationList,
};

pub struct NotificationManager {
    pub list: NotificationList,
    next_id: u64,
}

impl NotificationManager {
    pub fn new() -> Self {
        Self {
            list: NotificationList::new(),
            next_id: 1,
        }
    }

    pub fn notify(&mut self, kind: NotificationKind, title: impl Into<String>) {
        let id = self.next_id;
        self.next_id += 1;
        self.list
            .push(Notification::new(id, kind, title).ttl_ms(3_500));
    }

    pub fn notify_msg(
        &mut self,
        kind: NotificationKind,
        title: impl Into<String>,
        msg: impl Into<String>,
    ) {
        let id = self.next_id;
        self.next_id += 1;
        self.list.push(
            Notification::new(id, kind, title)
                .message(msg)
                .ttl_ms(5_000),
        );
    }

    pub fn tick(&mut self, dt: Duration) {
        self.list.tick(dt);
    }

    pub fn dismiss(&mut self, id: u64) {
        self.list.dismiss(id);
    }
}

impl Default for NotificationManager {
    fn default() -> Self {
        Self::new()
    }
}
