use fltk::{app, browser::HoldBrowser, enums::{Event, Key}, prelude::{BrowserExt, WidgetBase}};

use fltk::app::event_button;
use fltk::app::event_clicks;
use fltk::app::event_is_click;
use fltk::app::event_key;

use crate::{EntryType, Listing, Message};

pub struct MyBrowser {
    pub browser: HoldBrowser,
}

impl MyBrowser {
    pub fn new(tx: app::Sender<Message>) -> Self {
        let mut browser = HoldBrowser::default_fill();
        browser.set_column_widths(&[270,20]);

        browser.handle(move |widget, event| {
            match event {
                Event::Push => {
                    if event_is_click() {
                        if !widget.selected_items().is_empty() {
                            if event_clicks() && event_button() == 1 { //double or more clicks
                                tx.send(Message::OpenItem(widget.selected_items()[0])); // all this could be moved to a controller?
                                return true;
                            } else if event_button() == 3 {
                                tx.send(Message::UpDir(widget.selected_items()[0]));
                                return true;
                            }
                        } else if event_button() == 3 {
                            tx.send(Message::UpDir(1));
                            return true;
                        }
                    }
                    false// is this needed?
                },

                Event::KeyDown => {
                    match event_key() {
                        Key::Enter => {
                            if !widget.selected_items().is_empty() {
                                tx.send(Message::OpenItem(widget.selected_items()[0]));
                                return true;
                            }
                            false
                        },

                        Key::BackSpace => {
                            if !widget.selected_items().is_empty() {
                                tx.send(Message::UpDir(widget.selected_items()[0]));
                            } else {
                                tx.send(Message::UpDir(1)); //1 is the default
                            }
                            true
                        },

                        Key::Home => {
                            widget.select(1); //selecting non-excistent line appears to do nothing
                            true
                        },

                        Key::End => {
                            widget.select(widget.size());
                            true
                        },
                        Key::Escape => {
                            tx.send(Message::Quit);
                            true
                        },

                        _ => false, //end KeyDown match event_key
                    }
                },

                _ => false, // end match event
            }
        });

        Self {
            browser,
        }
    }

    pub fn populate_browser(&mut self, listing: &Vec<Listing>) {
        use human_bytes::human_bytes;
        self.browser.clear();

        if !listing.is_empty() {
            for l in listing {
                let type_prefix = match l.entry_type {
                    EntryType::Dir => "/",
                    EntryType::Link => "*",
                    EntryType::File => "",
                    EntryType::Archive => "/",
                    EntryType::Image => "",
                };
                let size = human_bytes(l.size as u32);
                self.browser.add(&format!("{}{}\t\t{}", type_prefix, &l.display_name, size));
            }
        }
    }
}