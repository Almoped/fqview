const PROGRAM_NAME: &str = "fqView";

use std::path::PathBuf;

mod my_app;
mod my_view;
mod my_display;
mod my_browser;
mod my_menu;
mod my_model;

use image::DynamicImage;

use crate::my_app::MyApp;

pub fn run(args: Vec<String>) {
    if let Ok(mut app) = MyApp::build(args) {
        app.launch();
    } else {
        println!("start path error");
    }
}

#[derive(Clone)]
pub enum Message {
    Quit,
    ToggleFullscreen,
    About,
    ShowListing(Vec<Listing>, PathBuf),
    OpenItem(i32),
    UpDir(i32),
    ImageDecoded(DynamicImage, PathBuf),
    WantToDisplay(PathBuf),
    ImageLoaded(PathBuf),
    StopImageDisplay,
    NextImage,
    PrevImage,
    GoLight,
    GoDark,
    ToggleStatusbar,
    Zoom1to1,
    ZoomFitToWindow,
    ToggleKeepAR,
    Info(String),
    UpdateStatusData,
    ZoomChanged,
}

#[derive(Ord, PartialOrd, PartialEq, Eq, Clone)]
pub enum EntryType {
    Dir,
    Link,
    File,
    Image, //make sub types of images and archives? which are also files..
    Archive,
}

 #[derive(Ord, PartialOrd, PartialEq, Eq, Clone)]
pub struct Listing {
    entry_type: EntryType,
    display_name: String,
    file_path: PathBuf,
    size: u64,
}

#[derive(Clone, Copy)]
pub struct ViewSettings {
    pub keep_ar: bool,
    pub fit_to_window: bool,
    pub zoom_lvl_x: f32,
    pub zoom_lvl_y: f32,
    pub centerpos_x: f32,
    pub centerpos_y: f32,
}

pub fn screen_center() -> (i32, i32) {
    (
        (fltk::app::screen_size().0 / 2.0) as i32,
        (fltk::app::screen_size().1 / 2.0) as i32,
    )
}