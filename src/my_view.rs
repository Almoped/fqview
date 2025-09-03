use std::collections::HashMap;
use std::error::Error;
use std::path::PathBuf;
use fltk::app::Sender;
use fltk::button::Button;
use fltk::dialog::message;
use fltk::frame::Frame;
use fltk::group::Flex;
use fltk::prelude::{BrowserExt, GroupExt, InputExt, WidgetBase, WidgetExt, WindowExt};
use fltk::group::FlexType::Column;
use fltk::{input, window};

use fltk_theme::widget_themes::OS_SPACER_THIN_DOWN_BOX;
use speedy2d::image::ImageHandle;

use crate::my_browser::MyBrowser;
use crate::my_display::MyDisplay;
use crate::my_menu::MyMenu;
use crate::{Listing, Message, PROGRAM_NAME};
use crate::ViewSettings;

pub struct MyView {
    main_win: window::Window,
    glut_row: Flex,
    pub status_row: Flex,
    middle_col: Flex,
    pub display: MyDisplay,
    browser_row: Flex,
    browser: MyBrowser,
    menu: MyMenu,
    inp_path: input::Input,
    pub want_to_display: Option<PathBuf>,
    vsettings: HashMap<ImageHandle, ViewSettings>, //maybe get vs pr pathbuf and serialize
    stat_display: bool,
    stat_messages: Frame,
    stat_zoomlvl: Frame,
    stat_gpu: Frame,
    stat_images: Frame,
    stat_data: Frame,
}

pub struct Statusline {
    messages: String,
    zoom_level: (i32, i32),
    gpu_data: usize,
    ram_data: usize,
}

impl MyView {
    pub fn build(tx: Sender<Message>) -> Self {
        let mut main_win = window::Window::default()
            .with_size(800, 600)
            .center_screen()
            .with_label(PROGRAM_NAME);

        let mut flex = Flex::default_fill().column();
            flex.set_spacing(0);
            let menu = MyMenu::new(&tx);
            
            /*if let Some(mut item) = menu.menu.find_item("&View/Keep image &aspect ratio\t") {
                item.set();
            }

            if let Some(mut item) = menu.menu.find_item("&View/Fit to &window\t") {
                item.set();
            }*/
            
        flex.fixed(&menu.menu, 25);

        let mut middle_col = Flex::default_fill();
            //middle_col.set_spacing(5);
            middle_col.set_type(Column);
            let mut browser_row = Flex::default_fill();
            browser_row.set_spacing(1);
                browser_row.set_type(Column);
                let mut row = Flex::default_fill();
                    row.set_spacing(1);
                    let mut btn_up = Button::default().with_label("@-38->");
                    btn_up.emit(tx, Message::UpDir(1));
                    row.fixed(&btn_up, 25);
                    let mut inp_path = input::Input::default();
                row.end();
                browser_row.fixed(&row, 25);
                let browser = MyBrowser::new(tx);
            browser_row.end();

            let mut glut_row = Flex::default_fill();                
                glut_row.set_type(Column);
                let glut_win = window::GlutWindow::default_fill();
                glut_win.end();
            glut_row.end();
            inp_path.set_value("yay"); //make this input reactive to input
            let mut status_row = Flex::default_fill().row(); //maybe flex is not the way to go about this
                let mut stat_messages = Frame::default().with_label("");
                stat_messages.set_align(fltk::enums::Align::Clip);
                
                    let mut spacer = Frame::default();
                    spacer.set_frame(OS_SPACER_THIN_DOWN_BOX);
                    status_row.fixed(&spacer, 2);

                let stat_zoomlvl = Frame::default().with_label("Zoom: 1.23 x 1.23");
                
                    let mut spacer = Frame::default();
                    spacer.set_frame(OS_SPACER_THIN_DOWN_BOX);
                    status_row.fixed(&spacer, 2);
                
                let mut stat_gpu = Frame::default().with_label("GPU: 4023 MiB");
                stat_gpu.set_tooltip("Size of textures loaded into video ram.");
                
                    let mut spacer = Frame::default();
                    spacer.set_frame(OS_SPACER_THIN_DOWN_BOX);
                    status_row.fixed(&spacer, 2);
                
                let mut stat_images = Frame::default().with_label("Images: 0123 GiB");
                stat_images.set_tooltip("Size of decoded images in ram.");
                
                    let mut spacer = Frame::default();
                    spacer.set_frame(OS_SPACER_THIN_DOWN_BOX);
                    status_row.fixed(&spacer, 2);

                let mut stat_data = Frame::default().with_label("Data: 1312 MiB");
                stat_data.set_tooltip("Size of cached data in ram.");
                
                status_row.fixed(&stat_zoomlvl, 150);
                status_row.fixed(&stat_gpu, 115);
                status_row.fixed(&stat_images, 135);
                status_row.fixed(&stat_data, 120);
                
                
            middle_col.fixed(&status_row, 18);
        middle_col.end();        
        flex.end();
        
        main_win.make_resizable(true);
        main_win.end();
        main_win.show();
        
        let display = MyDisplay::build(glut_win, tx);
        let want_to_display: Option<PathBuf> = None;
        let vsettings: HashMap<ImageHandle, ViewSettings> = HashMap::new();
       
        Self {
            main_win,
            browser_row,
            glut_row,
            status_row,
            middle_col,
            display,
            browser,
            menu,
            inp_path,
            want_to_display,
            vsettings,
            stat_display: true,
            stat_messages, //make a struct with these?
            stat_zoomlvl,
            stat_gpu,
            stat_images,
            stat_data,
          }
    }

    pub fn set_error_message(&mut self, e: Box<dyn Error>) {
        self.stat_messages.set_label(e.to_string().as_str());
    }
    
    pub fn set_stat_message(&mut self, s: &str) {
        self.stat_messages.set_label(s);
    }

    pub fn set_stat_zoomlvl(&mut self, s: &str) {
        self.stat_zoomlvl.set_label(s);
    }

    pub fn set_stat_gpu(&mut self, s: &str) {
        self.stat_gpu.set_label(s);
    }

    pub fn set_stat_images(&mut self, s: &str) {
        self.stat_images.set_label(s);
    }

    pub fn set_stat_data(&mut self, s: &str) {
        self.stat_data.set_label(s);
    }

    pub fn set_displaying_layout(&mut self) {
        self.browser_row.hide();
        self.glut_row.show();                            
        self.middle_col.layout();
        let _ = self.display.glut_win.take_focus();
    }

    pub fn set_browsing_layout(&mut self) {
        self.browser_row.show();
        self.glut_row.hide();
        self.middle_col.layout();
        let _ = self.browser.browser.take_focus();
    }

    pub fn populate_browser(&mut self, listing: &Vec<Listing>) { // move this into my_browser?
        self.browser.populate_browser(listing);
    }

    pub fn select_browser_item(&mut self, index: i32) {        
        self.browser.browser.select(index +1); //starts from 1
    }

    pub fn set_input_text(&mut self, pb: PathBuf) {        
        self.inp_path.set_value(&pb.display().to_string());
    }

    pub fn display_image(&mut self, handle: ImageHandle) {
        self.save_viewsettings();
        self.load_viewsettings(&handle);
        self.set_displaying_layout();
        *self.display.s_displaying_image.borrow_mut() = Some(handle);
        self.display.s_vc.borrow_mut().update_view_settings();
        self.update_window_label();
        self.display.glut_win.redraw();
    }

    pub fn load_viewsettings(&mut self, handle: &ImageHandle) {
        if self.vsettings.contains_key(handle) {
            self.display.put_vs(self.vsettings.remove(handle).expect("Vsettings contains the key."));
        } else {
            self.display.put_vs(ViewSettings {
                keep_ar: true, //look this up
                fit_to_window: true, //look this up
                zoom_lvl_x: 1.,
                zoom_lvl_y: 1.,
                centerpos_x: 0.5,
                centerpos_y: 0.5,
            });
        }
    }

    fn save_viewsettings(&mut self) { //maybe serialize pb and vs
        if let Some(handle) = self.display.s_displaying_image.borrow().as_ref() {
            self.vsettings.insert(handle.clone(), self.display.get_vs());
        }
    }

    pub fn stop_image_display(&mut self) {
        self.save_viewsettings();
        self.want_to_display = None;
        *self.display.s_displaying_image.borrow_mut() = None;
        self.set_browsing_layout();
        self.update_window_label();
    }

    pub fn update_window_label(&mut self) {
        if self.display.s_displaying_image.borrow().is_some() {
            if let Some(pb) = &self.want_to_display {
                let mut temp = pb.display().to_string();
                temp += " - ";
                temp += PROGRAM_NAME;
                self.main_win.set_label(&temp);
            }
        } else {
            self.main_win.set_label(PROGRAM_NAME);
        }
    }

    pub fn toggle_statusbar(&mut self) {
        self.stat_display = !self.stat_display;
        if self.stat_display {
            self.status_row.show();
        } else {
            self.status_row.hide();
        }
        self.middle_col.layout();
    }

    pub fn toggle_fs(&mut self) {
        if self.main_win.fullscreen_active() {
            self.main_win.fullscreen(false);
        } else {
            self.main_win.fullscreen(true);
        }
    }
}