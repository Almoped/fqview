use fltk::{enums::{FrameType, Shortcut}, menu, prelude::{MenuExt, WidgetExt}};

use crate::Message;
pub struct MyMenu {
    pub menu: menu::SysMenuBar, //pub for flex
}

impl MyMenu {
    pub fn new(tx: &fltk::app::Sender<Message>) -> Self {
        let mut menu = menu::SysMenuBar::default().with_size(80, 25);
        //menu.set_frame(FrameType::EngravedBox);

        menu.add_emit(
            "&File/&Open...\t",
            Shortcut::Ctrl | 'o',
            menu::MenuFlag::Normal,
            *tx,
            Message::ToggleFullscreen,
        );

        menu.add_emit(
            "&File/&Quit\t",
            Shortcut::Ctrl | 'q',
            menu::MenuFlag::Normal,
            *tx,
            Message::Quit,
        );

        menu.add_emit(
            "&View/Keep image &aspect ratio\t",
            Shortcut::Ctrl | 'a',
            menu::MenuFlag::Normal,
            *tx,
            Message::ToggleKeepAR, //make enum with toggles?
        );

        menu.add_emit(
            "&View/Fit to &window\t",
            Shortcut::None | '*',
            menu::MenuFlag::Normal,
            *tx,
            Message::ZoomFitToWindow,
        );

        menu.add_emit(
            "&View/Zoom &1 to 1\t",
            Shortcut::None | '/',
            menu::MenuFlag::Normal,
            *tx,
            Message::Zoom1to1,
        );

        menu.add_emit(
            "&View/&Fullscreen\t", //do some alt + enter, dbl_click?
            Shortcut::Ctrl | 'f',
            menu::MenuFlag::Normal,
            *tx,
            Message::ToggleFullscreen,
        );

        menu.add_emit(
            "&View/Statusbar\t",
            Shortcut::None,
            menu::MenuFlag::Normal,
            *tx,
            Message::ToggleStatusbar,
        );

        menu.add_emit(
            "&View/Theme/Light\t",
            Shortcut::None,
            menu::MenuFlag::Normal,
            *tx,
            Message::GoLight,
        );

        menu.add_emit(
            "&View/Theme/Dark\t",
            Shortcut::None,
            menu::MenuFlag::Normal,
            *tx,
            Message::GoDark,
        );

        menu.add_emit(
            "&Help/&About\t",
            Shortcut::None,
            menu::MenuFlag::Normal,
            *tx,
            Message::About,
        );

        Self { menu }
    }
}