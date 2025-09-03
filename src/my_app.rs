use std::{error::Error, io, path::PathBuf};

use fltk::{app::{self, Receiver, Sender}, prelude::WidgetExt};


use crate::{my_model::MyModel, my_view::MyView, screen_center, Message, PROGRAM_NAME};

pub struct Stats {
    statusbar: bool,
    data: usize,
    gpu: usize,
}

pub struct MyApp {
    app: app::App,
    tx: Sender<Message>,
    rx: Receiver<Message>,
    view: MyView,
    model: MyModel,
    start_path: PathBuf,
    gpu_data_size: usize,
    image_cache_size: usize,
}

impl MyApp {
    pub fn build(args: Vec<String>) -> Result<Self, Box<dyn Error>> {

        let first_arg =
        if args.len() > 1 {
            args[1].clone()
        } else {
            ".".to_owned()
        };

        let start_path = PathBuf::from(first_arg).canonicalize()?;
        //if first arg is an image: display it. register image and archive types?

        println!("start path: {:?}", start_path);

        let (tx, rx) = app::channel::<Message>();
        let app = app::App::default();
        let view = MyView::build(tx);
        let mut model = MyModel::build(tx, &start_path);
        
        tx.send(Message::GoDark);
        tx.send(Message::ShowListing(model.get_listing(&start_path)?, start_path.clone()));
        

        Ok(Self {
            app,
            tx,
            rx,
            view,
            model,
            start_path,
            gpu_data_size: 0,
            image_cache_size: 0,
        })
    }

    pub fn launch(&mut self) {
        while self.app.wait() {
            use Message::*;
            if let Some(msg) = self.rx.recv() {
                match msg {
                    ToggleFullscreen => {
                        self.view.toggle_fs();
                    }

                    ZoomChanged => {
                        let x = self.view.display.s_vc.borrow().get_x_zoom();
                        let y = self.view.display.s_vc.borrow().get_y_zoom();
                        let s = format!("Zoom: {:.2} x {:.2}", x, y);
                        self.view.set_stat_zoomlvl(&s);
                    }

                    UpdateStatusData => {
                        use human_bytes::human_bytes;
                        let g = self.gpu_data_size;
                        let i = self.image_cache_size;
                        let d = self.model.data_in_cache_size;
                        
                        let mut gg = String::from("GPU: ");
                        gg.push_str(human_bytes(g as f64).as_str());

                        let mut ii = String::from("Images: ");
                        ii.push_str(human_bytes(i as f64).as_str());

                        let mut dd = String::from("Data: ");
                        dd.push_str(human_bytes(d as f64).as_str());

                        self.view.set_stat_gpu(&gg);
                        self.view.set_stat_images(&ii);
                        self.view.set_stat_data(&dd);
                        self.view.status_row.layout();
                    }

                    ShowListing(listing, pb) => {
                        self.view.populate_browser(&listing);
                        self.view.set_input_text(pb);
                        self.view.set_browsing_layout();
                        self.tx.send(UpdateStatusData);
                    }

                    About => fltk::dialog::message(screen_center().0 - 300, screen_center().1 - 100,
                    format!("{PROGRAM_NAME} is a simple image viewer and could not be written without these:\n\n\tfltk-rs by Mohammed Alyousef\n\tarchive-reader\n\timage crate\n\tlibarchive\n\tSpeedy2D\n\tzip crate\n\tlibheif
                            \n{PROGRAM_NAME} is Copyright 2025 by Allan Pedersen
                            \nPermission is hereby granted, free of charge, to any person obtaining a copy of this software\nand associated documentation files (the “Software”), to deal in the Software without\nrestriction, including without limitation the rights to use, copy, modify, merge, publish,\ndistribute, sublicense, and/or sell copies of the Software, and to permit persons to whom the\nSoftware is furnished to do so, subject to the following conditions:
                            \nThe above copyright notice and this permission notice shall be included in all copies or\nsubstantial portions of the Software.
                            \nTHE SOFTWARE IS PROVIDED “AS IS”, WITHOUT WARRANTY OF ANY KIND, EXPRESS OR\nIMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,\nFITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL\nTHE AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR\nOTHER LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE,\nARISING FROM, OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR\nOTHER DEALINGS IN THE SOFTWARE.
                    ").as_str()), //make this include_bytes somehow
                    
                    Quit => {
                        println!("Graceful exit, goodbye.");
                        self.app.quit();
                    },

                    OpenItem(item_nr) => {
                        let res = self.model.open_item(item_nr);
                        if res.is_err() {
                            let e = res.unwrap_err();
                            self.view.set_error_message(e);
                        }
                    },

                    UpDir(_item_nr) => { //use item_nr when remembering last selection
                        self.model.goto_parent();
                    },

                    ImageDecoded(image, image_pb) => {
                        self.model.trying_to_load.remove(&image_pb);
                        if Some(image_pb.clone()) == self.view.want_to_display {
                            let img_size = image.as_bytes().len();
                            let res = self.view.display.s_vc.borrow_mut().upload_image(image);
                            match res {
                                Ok(handle) => {
                                    self.gpu_data_size += img_size;
                                    self.view.display_image(handle.clone());
                                    self.model.texture_cache.insert(image_pb, handle);
                                },

                                Err(e) => {
                                    self.view.set_error_message(e);
                                },
                            }
                        } else {
                            self.image_cache_size += image.as_bytes().len();
                            self.model.image_cache.insert(image_pb, image);
                            //store unshowed images in gpu or ram?
                        }
                        self.tx.send(UpdateStatusData);
                    },

                    WantToDisplay(image_pb) => {
                        self.view.want_to_display = Some(image_pb.clone());
                        if self.model.texture_cache.contains_key(&image_pb) {
                            if let Some(handle) = self.model.texture_cache.get(&image_pb) {
                                self.view.display_image(handle.clone());
                            }
                        } else if self.model.image_cache.contains_key(&image_pb) {
                            if let Some(image) = self.model.image_cache.remove(&image_pb) {
                                let img_size= image.as_bytes().len();                                
                                let res = self.view.display.s_vc.borrow_mut().upload_image(image);
                                match res {
                                    Ok(handle) => {
                                        self.gpu_data_size += img_size;
                                        self.image_cache_size -= img_size;
                                        self.view.display_image(handle.clone());
                                        self.model.texture_cache.insert(image_pb, handle);
                                    },

                                    Err(e) => {
                                        self.view.set_error_message(e);
                                    },
                                }
                            }
                        } else {
                            self.model.load_image_data(image_pb);
                        }
                        
                        if let Some(current) = self.view.want_to_display.clone() {
                            if let Some((next, _)) = self.model.get_next_image(current) {
                                self.model.load_image_data(next);                                
                            }
                        }
                        self.tx.send(UpdateStatusData);
                    },

                    StopImageDisplay => {                        
                        self.view.stop_image_display();
                    },

                    ImageLoaded(image_pb) => {
                        //if self.view.want_to_display == Some(image_pb.clone()) {
                            self.model.decode_image(image_pb);
                        //}
                        self.tx.send(UpdateStatusData);
                    },

                    NextImage => {
                        if let Some(current) = self.view.want_to_display.clone() {
                            if let Some((next, index)) = self.model.get_next_image(current) {
                                self.tx.send(WantToDisplay(next));
                                self.view.select_browser_item(index as i32);
                            }
                        }
                    },

                    PrevImage => {
                        if let Some(current) = self.view.want_to_display.clone() {
                            if let Some((prev, index)) = self.model.get_prev_image(current) {
                                self.tx.send(WantToDisplay(prev));
                                self.view.select_browser_item(index as i32);
                            }
                        }
                    },

                    GoLight => fltk_theme::ColorTheme::new(&fltk_theme::color_themes::fleet::LIGHT).apply(),
                    GoDark => fltk_theme::ColorTheme::new(&fltk_theme::color_themes::DARK_THEME).apply(),
                    ToggleStatusbar => self.view.toggle_statusbar(),

                    Zoom1to1 => { //make these couple to view, or something else, instead??
                        self.view.display.s_vc.borrow_mut().zoom_1_to_1();
                        self.view.display.glut_win.redraw();
                    },

                    ZoomFitToWindow => {
                        self.view.display.s_vc.borrow_mut().zoom_fit_to_window();
                        self.view.display.glut_win.redraw();
                    },
                    
                    ToggleKeepAR => {
                        let temp = self.view.display.s_vc.borrow().keep_ar;
                        self.view.display.s_vc.borrow_mut().keep_ar = !temp;
                        self.view.display.s_vc.borrow_mut().update_view_settings();
                        self.view.display.glut_win.redraw();
                    },

                    Info(s) => {
                        self.view.set_stat_message(&s);
                    }
                }
            }
        }
    }

}

