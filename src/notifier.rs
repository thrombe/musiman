

pub struct Notifier {
    // TODO
    // use some kinda queue. maybe a channel?
    pub notifs: Vec<String>,
}

impl Notifier {
    pub fn new() -> Self {
        Self {
            notifs: vec![],
        }
    }
}

