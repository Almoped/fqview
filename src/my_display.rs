use std::{cell::RefCell, error::Error, rc::Rc};

use fltk::{app::{self, event_button, event_dx_value, event_dy_value, event_key, event_key_down, event_state}, enums::{Event, Key}, prelude::{WidgetBase, WidgetExt}, window::{self, GlutWindow}};
use image::{DynamicImage, EncodableLayout};
use speedy2d::{dimen::Vector2, image::ImageHandle, shape::Rectangle};

use crate::Message;
use crate::ViewSettings;

pub struct MyDisplay {
    pub glut_win: window::GlutWindow,
    pub(crate)s_displaying_image: Rc<RefCell<Option<ImageHandle>>>,
    pub s_vc: Rc<RefCell<ViewConfig>>,
}

impl MyDisplay {
    pub fn build(mut glut_win: GlutWindow, tx: app::Sender<Message>) -> Self {
        glut_win.show();
        
        let glut_w = glut_win.pixel_w();
        let glut_h = glut_win.pixel_h();
        
        let renderer = unsafe {
            speedy2d::GLRenderer::new_for_gl_context((glut_w as u32, glut_h as u32), |fn_name| {
            glut_win.get_proc_address(fn_name) as *const _
        })}.expect("cannot connect glcontext");
        println!("renderer ok");

        let s_renderer: Rc<RefCell<speedy2d::GLRenderer>> = Rc::from(RefCell::from(renderer));
        let s_displaying_image: Rc<RefCell<Option<ImageHandle>>> = Rc::from(RefCell::from(None));
        let s_position_in_vp = Rc::new(RefCell::new(Rectangle::new(Vector2::new(0., 0.), Vector2::new(200., 200.))));
        let s_image_coords = Rc::new(RefCell::new(Rectangle::new(Vector2::new(0., 0.), Vector2::new(1., 1.))));
        let s_centerpos_x = Rc::new(RefCell::new(0.5));
        let s_centerpos_y = Rc::new(RefCell::new(0.5));

        //using these variables in callbacks and methods
        let vc = ViewConfig {
            glut_win: glut_win.clone(),
            s_renderer: s_renderer.clone(),
            s_position_in_vp: s_position_in_vp.clone(),
            s_image_coords: s_image_coords.clone(),
            s_displaying_image: s_displaying_image.clone(),
            zoom_lvl_x_effective: 1.,
            zoom_lvl_y_effective: 1.,
            onepix_modifier_x: 1.,
            onepix_modifier_y: 1.,
            min_visible_xpos: 1.,
            min_visible_ypos: 1.,
            max_visible_xpos: 1.,
            max_visible_ypos: 1.,

            keep_ar: true,
            fit_to_window: true,
            zoom_lvl_x: 1.,
            zoom_lvl_y: 1.,
            s_centerpos_x: s_centerpos_x.clone(),
            s_centerpos_y: s_centerpos_y.clone(),
            
            tx: tx,
        };

        let s_vc = Rc::new(RefCell::new(vc));

        glut_win.draw({
            let ren = s_renderer.clone();
            let displaying_image = s_displaying_image.clone();
            let position_in_vp = s_position_in_vp.clone();
            let image_coords = s_image_coords.clone();
            
            move |widget| {
                if let Some(handle) = displaying_image.borrow().as_ref() {
                    ren.borrow_mut().set_viewport_size_pixels(Vector2::new(widget.pixel_w() as u32, widget.pixel_h() as u32));
                    ren.borrow_mut().draw_frame(|graphics| {
                        graphics.clear_screen(speedy2d::color::Color::DARK_GRAY);
                        
                        graphics.draw_rectangle_image_subset_tinted(
                            position_in_vp.borrow().as_ref(),
                            speedy2d::color::Color::WHITE,
                            image_coords.borrow().as_ref(),
                            handle);
                    });
                }
            }
        });

        let movement_vec: Rc<RefCell<Vector2<f32>>> = Rc::new(RefCell::new(Vector2::new(0.,0.)));
        
        app::add_idle3({
            let mut widget = glut_win.clone();
            let m_v = movement_vec.clone();
            let vvc = s_vc.clone();
            move |_| {
                if m_v.borrow().x != 0. || m_v.borrow().y != 0. {
                    //following four lines: if scrolling away from an edge with a centerpos beyond whats a visible change, start from the visible position
                    if m_v.borrow().y < 0. && *vvc.borrow().s_centerpos_y.borrow() > vvc.borrow().max_visible_ypos {*vvc.borrow().s_centerpos_y.borrow_mut() = vvc.borrow().max_visible_ypos}
                    if m_v.borrow().y > 0. && *vvc.borrow().s_centerpos_y.borrow() < vvc.borrow().min_visible_ypos {*vvc.borrow().s_centerpos_y.borrow_mut() = vvc.borrow().min_visible_ypos}
                    if m_v.borrow().x < 0. && *vvc.borrow().s_centerpos_x.borrow() > vvc.borrow().max_visible_xpos {*vvc.borrow().s_centerpos_x.borrow_mut() = vvc.borrow().max_visible_xpos}
                    if m_v.borrow().x > 0. && *vvc.borrow().s_centerpos_x.borrow() < vvc.borrow().min_visible_xpos {*vvc.borrow().s_centerpos_x.borrow_mut() = vvc.borrow().min_visible_xpos}
                    
                    *vvc.borrow().s_centerpos_x.borrow_mut() += m_v.borrow().x;
                    *vvc.borrow().s_centerpos_y.borrow_mut() += m_v.borrow().y;
                    
                    vvc.borrow_mut().update_view_settings();
                    widget.redraw();
                }
                app::sleep(0.007);                                            
            }
        });

        let mut button1_down = false;

        
        glut_win.handle( {
            let mut click_coords = (0, 0);
            let mut start_pos = (0., 0.);
            let vvc = s_vc.clone();

            move |widget, event| {
            match event {
                Event::Focus => {
                    true
                },

                Event::Resize => {                    
                    vvc.borrow_mut().update_view_settings();
                    true
                },

                Event::Push => {
                    if event_button() == 1 { button1_down = true}
                    click_coords = app::event_coords();
                    //following four lines: clicking the image makes it possible to begin dragging with a visible change, move these to event::drag as single fire??
                    if *vvc.borrow().s_centerpos_y.borrow() > vvc.borrow().max_visible_ypos {*vvc.borrow().s_centerpos_y.borrow_mut() = vvc.borrow().max_visible_ypos}
                    if *vvc.borrow().s_centerpos_y.borrow() < vvc.borrow().min_visible_ypos {*vvc.borrow().s_centerpos_y.borrow_mut() = vvc.borrow().min_visible_ypos}
                    if *vvc.borrow().s_centerpos_x.borrow() > vvc.borrow().max_visible_xpos {*vvc.borrow().s_centerpos_x.borrow_mut() = vvc.borrow().max_visible_xpos}
                    if *vvc.borrow().s_centerpos_x.borrow() < vvc.borrow().min_visible_xpos {*vvc.borrow().s_centerpos_x.borrow_mut() = vvc.borrow().min_visible_xpos}

                    start_pos = (*vvc.borrow().s_centerpos_x.borrow(), *vvc.borrow().s_centerpos_y.borrow());                    
                    true

                    
                }

                Event::Drag => {
                    let dx = app::event_coords().0 - click_coords.0;
                    let dy = app::event_coords().1 - click_coords.1;
                    //images drawn as quards have reversed coordinates 
                    *vvc.borrow().s_centerpos_x.borrow_mut() = start_pos.0 - vvc.borrow().onepix_modifier_x * dx as f32;
                    *vvc.borrow().s_centerpos_y.borrow_mut() = start_pos.1 - vvc.borrow().onepix_modifier_y * dy as f32;
                    vvc.borrow_mut().update_view_settings();
                    widget.redraw();
                    true
                }

                Event::Released => {
                    if event_button() == 1 { button1_down = false}
                    true
                }

                Event::MouseWheel => {
                    if button1_down {
                        if event_dy_value() > 0 {
                            vvc.borrow_mut().zoom_out();
                            widget.redraw();
                        } else if event_dy_value() < 0 {
                            vvc.borrow_mut().zoom_in();
                            widget.redraw();
                        }
                    } else {
                        if event_dy_value() > 0 {
                            tx.send(Message::NextImage);
                        } else if event_dy_value() < 0 {
                            tx.send(Message::PrevImage);
                        }
                    }
                    //let a  = event_state(); // use this with bitflags for modifiers?
                    println!("wheel action, dx_value(): {}, dy_value(): {}", event_dx_value(), event_dy_value());
                    true
                }

                Event::KeyUp => {
                    match event_key() {
                        Key::Up | Key::Down | Key::Left | Key::Right => {
                            if !event_key_down(Key::Up)
                            && !event_key_down(Key::Down)
                            && !event_key_down(Key::Left)
                            && !event_key_down(Key::Right) {
                                *movement_vec.borrow_mut() = Vector2::new(0.,0.);
                            }
                            true
                        },

                        _ => false,
                    }
                }

                Event::KeyDown => {
                    match event_key() {
                        Key::Up | Key::Down | Key::Left | Key::Right => {
                            if !event_key_down(Key::Up)
                                && !event_key_down(Key::Down)
                                && !event_key_down(Key::Left)
                                && !event_key_down(Key::Right) {
                                    *movement_vec.borrow_mut() = Vector2::new(0.,0.);
                            } else {
                                let mut local_mv = Vector2::new(0.,0.);
                                if event_key_down(Key::Up) {
                                    local_mv.y -= vvc.borrow().onepix_modifier_y;
                                }
                                if event_key_down(Key::Down) {
                                    local_mv.y += vvc.borrow().onepix_modifier_y;
                                }
                                if event_key_down(Key::Left) {
                                    local_mv.x -= vvc.borrow().onepix_modifier_x;
                                }
                                if event_key_down(Key::Right) {
                                    local_mv.x += vvc.borrow().onepix_modifier_x;
                                }
                                if !event_key_down(Key::ShiftL) && !event_key_down(Key::ShiftR) {
                                    local_mv.x *= 5.;
                                    local_mv.y *= 5.;
                                }
                                if event_key_down(Key::ControlL) || event_key_down(Key::ControlR) {
                                    local_mv.x *= 3.;
                                    local_mv.y *= 3.;
                                }
                                if local_mv.x != 0. && local_mv.y != 0. {
                                    local_mv.x *= 0.707;
                                    local_mv.y *= 0.707;
                                }

                                movement_vec.borrow_mut().x = local_mv.x;
                                movement_vec.borrow_mut().y = local_mv.y;
                            }
                            true //was up, down, left or right
                        },

                        Key::Pause => {
                            vvc.borrow_mut().zoom_in();
                            widget.redraw();
                            true
                        },

                        Key::ScrollLock => {
                            vvc.borrow_mut().zoom_out();
                            widget.redraw();
                            true
                        },

                        Key::Insert => {
                            vvc.borrow_mut().zoom_1_to_1();
                            widget.redraw();
                            true
                        },

                        Key::Delete => {
                            vvc.borrow_mut().zoom_fit_to_window();
                            widget.redraw();
                            true
                        },

                        Key::Enter => {
                            tx.send(Message::StopImageDisplay);
                            true
                        },

                        Key::Escape => {
                            tx.send(Message::StopImageDisplay);
                            true
                        },

                        Key::PageUp => {
                            tx.send(Message::PrevImage);
                            true
                        },

                        Key::PageDown => {
                            tx.send(Message::NextImage);
                            true
                        },
                        
                        _ => {
                            if let Some(cha) = event_key().to_char() { //can't seem to match '/'
                                match cha {
                                    ' ' => {
                                        println!("/");
                                        true
                                    }

                                    'b' => {
                                        println!("b");
                                        true
                                    }
                                    _ => false
                                }
                            } else {
                                false
                            }
                        }, 
                        
                    }
                }, //end Event::KeyDown

                _ => { false },
            }
        }});

        Self {
            glut_win,
            s_displaying_image,
            s_vc,
        }
    }

    pub fn put_vs(&mut self, vs: ViewSettings) {
        self.s_vc.borrow_mut().keep_ar = vs.keep_ar;
        self.s_vc.borrow_mut().fit_to_window = vs.fit_to_window;
        self.s_vc.borrow_mut().zoom_lvl_x = vs.zoom_lvl_x;
        self.s_vc.borrow_mut().zoom_lvl_y = vs.zoom_lvl_y;
        *self.s_vc.borrow().s_centerpos_x.borrow_mut() = vs.centerpos_x;
        *self.s_vc.borrow().s_centerpos_y.borrow_mut() = vs.centerpos_y;
    }

    pub fn get_vs(&self) -> ViewSettings {
        ViewSettings {
            keep_ar: self.s_vc.borrow().keep_ar,
            fit_to_window: self.s_vc.borrow().fit_to_window,
            zoom_lvl_x: self.s_vc.borrow().zoom_lvl_x,
            zoom_lvl_y: self.s_vc.borrow().zoom_lvl_y,
            centerpos_x: *self.s_vc.borrow().s_centerpos_x.borrow(),
            centerpos_y: *self.s_vc.borrow().s_centerpos_y.borrow(),
        }
    }
}

pub struct ViewConfig {
    glut_win: GlutWindow,
    s_renderer: Rc<RefCell<speedy2d::GLRenderer>>,
    s_position_in_vp: Rc<RefCell<Rectangle>>,
    s_image_coords: Rc<RefCell<Rectangle>>,
    s_displaying_image: Rc<RefCell<Option<ImageHandle>>>,
    zoom_lvl_x_effective: f32,
    zoom_lvl_y_effective: f32,
    onepix_modifier_x: f32,
    onepix_modifier_y: f32,
    min_visible_xpos: f32,
    min_visible_ypos: f32,
    max_visible_xpos: f32,
    max_visible_ypos: f32,
    pub keep_ar: bool,
    fit_to_window: bool,
    zoom_lvl_x: f32,
    zoom_lvl_y: f32,
    s_centerpos_x: Rc<RefCell<f32>>,
    s_centerpos_y: Rc<RefCell<f32>>,
    tx: app::Sender<Message>,
}

impl ViewConfig {
    pub fn get_x_zoom(&self) -> f32 {
        if self.fit_to_window {
            self.zoom_lvl_x_effective
        } else {
            self.zoom_lvl_x
        }
    }

    pub fn get_y_zoom(&self) -> f32 {
        if self.fit_to_window {
            self.zoom_lvl_y_effective
        } else {
            self.zoom_lvl_y
        }
    }
    
    pub fn upload_image(&self, image: DynamicImage) -> Result<ImageHandle, Box<dyn Error>> { //make this result, incase upload fails
        println!("uploading an image");
        match image {
            DynamicImage::ImageRgb8(image_buffer) => {
                Ok(self.s_renderer.borrow_mut().create_image_from_raw_pixels(
                speedy2d::image::ImageDataType::RGB, 
                speedy2d::image::ImageSmoothingMode::Linear, //set smoothingmode from ui
                Vector2::new(image_buffer.width(), image_buffer.height()),
                image_buffer.as_bytes())?)
            },

            DynamicImage::ImageRgba8(image_buffer) => {
                Ok(self.s_renderer.borrow_mut().create_image_from_raw_pixels(
                speedy2d::image::ImageDataType::RGBA, 
                speedy2d::image::ImageSmoothingMode::Linear,
                Vector2::new(image_buffer.width(), image_buffer.height()),
                image_buffer.as_bytes())?)
            },

            _ => {
                Ok(self.s_renderer.borrow_mut().create_image_from_raw_pixels(
                speedy2d::image::ImageDataType::RGB, 
                speedy2d::image::ImageSmoothingMode::Linear,
                Vector2::new(image.width(), image.height()),
                image.into_rgb8().as_bytes())?)
            },
        }
    }

    pub fn update_view_settings(&mut self) {
        if let Some(handle) = self.s_displaying_image.borrow().as_ref() {
            let image_size= handle.size();
            let i_w = image_size.x as f32;
            let i_h = image_size.y as f32;
            let w_w = self.glut_win.width() as f32;
            let w_h = self.glut_win.height() as f32;
            
            let image_ar = i_w/i_h;
            
            let zoom_x = self.zoom_lvl_x;
            let zoom_y = self.zoom_lvl_y;
            
            let mut new_i_w;
            let mut new_i_h;

            // zoom level
            if self.keep_ar {
                new_i_w = i_w*(zoom_x+zoom_y)/2.;
                new_i_h = i_h*(zoom_x+zoom_y)/2.;
            } else {
                new_i_w = i_w*zoom_x;
                new_i_h = i_h*zoom_y;
            }

            //fit to window
            if self.fit_to_window {
                new_i_w = w_w;
                new_i_h = w_h;
            }

            //keep ar
            if self.keep_ar {
                if image_ar * new_i_h < new_i_w {
                    new_i_w = image_ar * new_i_h;
                } else {
                    new_i_h = new_i_w / image_ar;
                }
            }

            //crop and zoom in
            let mut xzl= 1.;
            let mut yzl= 1.;
            
            if new_i_w > w_w {
                xzl = w_w / new_i_w;
                new_i_w = w_w;
            }

            if new_i_h > w_h {
                yzl = w_h / new_i_h;
                new_i_h = w_h;
            }

            //move all these calculations elsewhere?
            self.zoom_lvl_x_effective = new_i_w / i_w;
            self.zoom_lvl_y_effective = new_i_h / i_h;
            
            self.onepix_modifier_x = xzl/w_w;
            self.onepix_modifier_y = yzl/w_h;

            self.min_visible_xpos = xzl/2.;
            self.min_visible_ypos = yzl/2.;

            self.max_visible_xpos = 1. - xzl/2.;
            self.max_visible_ypos = 1. - yzl/2.;

            
            //center in window
            let mut x_padding = 0.;
            let mut y_padding = 0.;

            if new_i_w < w_w {
                x_padding = ((w_w - new_i_w)/2.).floor();
            }
            
            if new_i_h < w_h {
                y_padding = ((w_h - new_i_h)/2.).floor();
            }
            
            let tl = Vector2::new(0.+x_padding, 0.+y_padding);
            let br = Vector2::new(new_i_w+x_padding, new_i_h+y_padding);
            let position_in_vp = Rectangle::new(tl,br);
            *self.s_position_in_vp.borrow_mut() = position_in_vp;

            //don't try to scroll beyond image border
            if *self.s_centerpos_x.borrow() > 1. {*self.s_centerpos_x.borrow_mut() = 1.}
            if *self.s_centerpos_x.borrow() < 0. {*self.s_centerpos_x.borrow_mut() = 0.}
            if *self.s_centerpos_y.borrow() > 1. {*self.s_centerpos_y.borrow_mut() = 1.}
            if *self.s_centerpos_y.borrow() < 0. {*self.s_centerpos_y.borrow_mut() = 0.}
            

            //zoom and pos
            let mut spx = *self.s_centerpos_x.borrow() - xzl/2.;
            let mut spy = *self.s_centerpos_y.borrow() - yzl/2.;
            if 0. > spx {spx = 0.}
            if 0. > spy {spy = 0.}
            
            let spx_max = 1. - xzl;
            let spy_max = 1. - yzl;
            if spx > spx_max { spx = spx_max}
            if spy > spy_max { spy = spy_max}

            let image_coords = Rectangle::new(Vector2::new(spx, spy), Vector2::new(spx+xzl, spy+yzl));
            *self.s_image_coords.borrow_mut() = image_coords;
            self.tx.send(Message::ZoomChanged);
        }
    }

    pub fn disable_fit_to_window(&mut self) {
        self.zoom_lvl_x = self.zoom_lvl_x_effective;
        self.zoom_lvl_y = self.zoom_lvl_y_effective;
        self.fit_to_window = false;
    }

    pub fn zoom_in(&mut self) { //make zoom range function?
        if self.fit_to_window {self.disable_fit_to_window();}
        self.zoom_lvl_x += self.zoom_lvl_x * 10./100.;
        self.zoom_lvl_y += self.zoom_lvl_y * 10./100.;
        self.update_view_settings();
    }

    pub fn zoom_out(&mut self) {
        if self.fit_to_window {self.disable_fit_to_window();}
        self.zoom_lvl_x -= self.zoom_lvl_x * 10./110.;
        self.zoom_lvl_y -= self.zoom_lvl_y * 10./110.;
        self.update_view_settings();
    }

    pub fn zoom_fit_to_window(&mut self) {
        self.fit_to_window = true;
        self.update_view_settings();
    }

    pub fn zoom_1_to_1(&mut self) {
        self.zoom_lvl_x = 1.;
        self.zoom_lvl_y = 1.;
        self.fit_to_window = false;
        self.update_view_settings();
    }
}