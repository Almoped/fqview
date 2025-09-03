const DEFAULT_BROWSER_SELECTION: i32 = 1;
const IMAGE_EXTENSIONS: [&str; 18] = ["bmp", "dds", "ff", "gif", "hdr", "ico", "jpg", "jpeg", "exr", "png", "pnm", "qoi", "tga", "tif", "tiff", "webp", "heic", "heif"];
const ARCHIVE_EXTENSIONS: [&str; 12] = ["iso", "zip", "7z", "cab", "rar", "xar", "lzh", "lha", "gz", "bz2", "zst", "jar"]; //lzma??

use std::{collections::{HashMap, HashSet}, error::Error, fs::{self, File}, io::{self, BufReader, Cursor, ErrorKind, Read}, path::{Component, Path, PathBuf}};

use archive_reader::Archive;
use archive_reader::error::Result;

use fltk::app::Sender;
use image::{DynamicImage, ImageReader};
use speedy2d::image::ImageHandle;

use libheif_rs::HeifContext;
use libheif_rs::LibHeif;


use crate::{EntryType, Listing, Message};

pub struct MyModel {
    tx: Sender<Message>,
    cwd: PathBuf,
    listings: HashMap<PathBuf, Vec<Listing>>, //cached directory and archive listings
    data_cache: HashMap<PathBuf,Vec<u8>>, //archives and compressed images
    pub image_cache: HashMap<PathBuf, DynamicImage>, //decompressed images
    pub texture_cache: HashMap<PathBuf, ImageHandle>, //images on gpu
    pub trying_to_load: HashSet<PathBuf>, //trying to load these
    pub(crate) data_in_cache_size: usize,
}

impl MyModel {
    pub fn build(tx: Sender<Message>, start_path: &Path) -> Self {

        let cwd = PathBuf::from(start_path);
        let listings: HashMap<PathBuf, Vec<Listing>> = HashMap::new();
        let data_cache: HashMap<PathBuf,Vec<u8>> = HashMap::new();
        let image_cache: HashMap<PathBuf, DynamicImage> = HashMap::new();
        let texture_cache: HashMap<PathBuf, ImageHandle> = HashMap::new();
        let trying_to_load: HashSet<PathBuf> = HashSet::new();

        Self {
            tx,
            cwd,
            listings,
            data_cache,
            image_cache,
            texture_cache,
            trying_to_load,
            data_in_cache_size: 0,
          }
    }

    pub fn goto_parent(&mut self) -> Result<(), Box<dyn Error>> {
        if let Some(parent) = self.cwd.parent() {
            let new_path = PathBuf::from(parent);
            let new_listing = self.get_listing(&new_path)?;
            self.tx.send(Message::ShowListing(new_listing, new_path));
        }
        Ok(())
    }

    pub fn open_item(&mut self, browser_item_nr: i32) -> Result<(), Box<dyn Error>> {
        if let Some(current_listing) = self.listings.get(&self.cwd) {
            if current_listing.len() >= browser_item_nr as usize {
                let selected = current_listing[browser_item_nr as usize-1].clone(); //browser is 1 based, vector 0 based
                match selected.entry_type {
                    EntryType::Dir => {
                        let new_listing = self.get_listing(&selected.file_path)?;
                        self.tx.send(Message::ShowListing(new_listing, selected.file_path));
                        Ok(())
                    },

                    EntryType::Archive => {
                        let new_listing = self.get_listing(&selected.file_path)?;
                        self.tx.send(Message::ShowListing(new_listing, selected.file_path));
                        Ok(())
                    },

                    EntryType::Link => (Ok(())), //lookup what link points too somehow
                    
                    EntryType::File => {Ok(())}, //don't do much if not img or archive, alt/ctrl click to force type?
                    
                    EntryType::Image => {
                        self.tx.send(Message::WantToDisplay(selected.file_path));
                        Ok(())
                    },
                }
            } else {
                Ok(()) //make this an error
            }
        } else {
            Ok(()) //also make this an error
        }
    }

    //if not image in texture_cache, decoded_image_cache, data_cache load from disk or archive
    pub fn load_image_data(&mut self, image_pb: PathBuf) {
        if self.texture_cache.contains_key(&image_pb) {return;}
        if self.image_cache.contains_key(&image_pb) {return;}

        if self.data_cache.contains_key(&image_pb) {
            self.tx.send(Message::ImageLoaded(image_pb));
            return;
        }

        if self.trying_to_load.contains(&image_pb) {
            return;
        } else {
            self.trying_to_load.insert(image_pb.clone());
        }

        if let Some((main, maybe_sub)) = self.locate_resource(&image_pb) {            
            if let Some(sub) = maybe_sub { //we're fetching from inside an archive
                //println!("extract: {:?} from {:?}", sub, main);
                if let Ok(true) = main.as_path().try_exists() {
                    self.extract_from_archive(&main, &sub);
                } else {
                    self.extract_from_data_cache(&main, &sub);
                }
            } else {
                //open from fs
                //println!("open from fs: {:?}", main);
                if let Ok(f) = File::open(main) {
                    let mut buf_read = BufReader::new(f);
                    let mut data = vec![];
                    if let Ok(size) = buf_read.read_to_end(&mut data) { //reading all data, image or not
                        if size > 0 {
                            self.data_in_cache_size += data.len();
                            self.data_cache.insert(image_pb.clone(), data);
                        }
                    }
                }
            }
        }
        
        if self.data_cache.contains_key(&image_pb) {
            self.tx.send(Message::ImageLoaded(image_pb));
        }
    }

    pub fn decode_image(&mut self, image_pb: PathBuf) {
        if let Some(image_data) = self.data_cache.remove(&image_pb) {
            self.data_in_cache_size -= image_data.len();
            let tx = self.tx;

            std::thread::spawn(move || { //use more controlled threading, lookup builder mutex saturate
                let res = MyModel::try_decode_image(tx, image_data, image_pb.to_path_buf());
                if res.is_err() {
                    tx.send(Message::Info(String::from(format!("Problem decoding image, {:?}", image_pb))));
                }
            });
        }
    }

    fn try_decode_image(tx: Sender<Message>, image_data: Vec<u8>, image_pb: PathBuf) -> Result<bool, Box<dyn Error>>{
        if let Some(ex) = image_pb.extension() {            
            if ex.eq_ignore_ascii_case("heic") || ex.eq_ignore_ascii_case("heif") {
                let lib_heif = LibHeif::new();
                let ctx = HeifContext::read_from_bytes(&image_data)?;
                let handle = ctx.primary_image_handle()?;
                let has_alpha = handle.has_alpha_channel();
                let color_space = if has_alpha {
                    libheif_rs::ColorSpace::Rgb(libheif_rs::RgbChroma::Rgba)
                } else {
                    libheif_rs::ColorSpace::Rgb(libheif_rs::RgbChroma::Rgb)
                };
            
                if let Ok(img) = lib_heif.decode(&handle, color_space, None) {
                    if let Some(inter) = img.planes().interleaved {
                        if has_alpha {
                            if let Some(buf) = image::ImageBuffer::from_vec(inter.width, inter.height, inter.data.to_vec()) {
                                let image = DynamicImage::ImageRgba8(buf);
                                tx.send(Message::ImageDecoded(image, image_pb));
                                return Ok(true);
                            }
                        } else { //no alpha
                            if let Some(buf) = image::ImageBuffer::from_vec(inter.width, inter.height, inter.data.to_vec()) {
                                let image = DynamicImage::ImageRgb8(buf);
                                tx.send(Message::ImageDecoded(image, image_pb));
                                return Ok(true);
                            }
                        }
                    }
                }
            } else { //not heic
                let buf_read = Cursor::new(image_data);
                let maybe_image = ImageReader::new(buf_read);
                if let Ok(img) = maybe_image.with_guessed_format() {
                    let image = img.decode()?;
                    tx.send(Message::ImageDecoded(image, image_pb));
                    return Ok(true);
                }
            }
        }
        Ok(false) //this should err out somehow
    }

    fn locate_resource(&mut self, path: &Path) -> Option<(PathBuf, Option<PathBuf>)> {
        if let Ok(true) = path.try_exists() {
            return Some((path.to_path_buf(), None));
        }
        if path.parent().is_some() {
            let mut maybe_archive = path.components();
            let mut parts = Vec::new();
            let mut found = false;

            while let Some(std::path::Component::Normal(part)) = maybe_archive.next_back() {
                parts.push(part);
                if let Ok(true) = maybe_archive.as_path().try_exists() {
                    found = true;
                    break;                            
                }
                if self.data_cache.contains_key(maybe_archive.as_path()) {                            
                    found = true;
                    break;                            
                }
                //what if next_back runs out??
            }

            parts.reverse();

            let mut sub_path = PathBuf::new();
            for part in parts {
                let tmp = sub_path.join(part);
                sub_path = tmp;
            }
            if found {
                return Some((maybe_archive.as_path().to_path_buf(), Some(sub_path)))
            }
        }
        None
    }

    pub fn get_listing(&mut self, path: &Path) -> Result<Vec<Listing>, Box<dyn Error>> {
        if self.listings.contains_key(path) {
            println!("using cached listing");
            self.cwd = path.to_path_buf();
            return Ok(self.listings.get(path).expect("Path was found in listings.").clone());
        }
        match path.try_exists() {
            Ok(false) => {
                println!("not found on fs");
            },

            Ok(true) => {
                if path.is_file() {
                    let list = MyModel::get_filelist(path)?;
                    self.add_filelist_to_directory(list, path.to_path_buf());
                    self.cwd = path.to_path_buf();
                } else if path.is_dir() {
                    self.listings.insert(path.to_path_buf(), MyModel::list_dir(path)?); //maybe move this
                    self.cwd = path.to_path_buf();
                }
                println!("path exists on fs");
                return Ok(self.listings.get(path).expect("This path should have just been added to listings.").clone());
            },

            Err(err) if err.kind() == ErrorKind::NotADirectory => {
                println!("maybe archive path inside another archive");
                if let Some(sub_ext) = path.extension() {
                    if sub_ext.eq_ignore_ascii_case("zip") { //can only list zipfiles from other archives
                        if let Some((maybe_archive, Some(sub_path))) = self.locate_resource(path) {
                            if let Ok(true) = maybe_archive.try_exists() {
                                //extract from archive in fs and list
                                let mut maybe_is_archive = false;
                                if let Some(maybe_ext) = maybe_archive.extension() {
                                    for archex in ARCHIVE_EXTENSIONS {
                                        if maybe_ext.eq_ignore_ascii_case(archex) {
                                            maybe_is_archive = true;
                                            break;
                                        }
                                    }
                                }
                                if maybe_is_archive {
                                    self.extract_from_archive(maybe_archive.as_path(), &sub_path);
                                }
                            } else { //found in cache
                                self.extract_from_data_cache(maybe_archive.as_path(), &sub_path);
                            }

                            if self.data_cache.contains_key(path) {
                                if let Some(a) = self.data_cache.get(path) {
                                    let list = MyModel::get_zip_filelist(a.clone()); //this clone is no good
                                    self.add_filelist_to_directory(list, path.to_path_buf());
                                    self.cwd = path.to_path_buf();
                                    return Ok(self.listings.get(path).expect("Path was found in listings.").clone());
                                }
                            }
                        }
                    }
                }
            }

            Err(err) => {
                println!("other error");
                println!("kind: {}", err.kind());
            },
        }

        let yay: Vec<Listing> = vec![
            Listing {
                entry_type: EntryType::File,
                display_name: String::from("not yay"),
                file_path: PathBuf::from("1"),
                size: 666
            }
        ];
        self.cwd = path.to_path_buf();
        Ok(yay)
    }

    fn extract_from_archive(&mut self, archive_path: &Path, sub_path: &Path) {
        if let Some(ex) = archive_path.extension() {
            if ex.eq_ignore_ascii_case("zip") { //use zip
                let f = File::open(archive_path).expect("unable to load file");
                let buf_read = BufReader::new(f);
                let mut archive = zip::ZipArchive::new(buf_read).unwrap();
                if let Ok(mut file) = archive.by_name(sub_path.to_str().unwrap()) {
                    let mut content = vec![];
                    if let Ok(_size) = io::copy(&mut file, &mut content) {
                        let name = archive_path.join(sub_path);
                        self.data_in_cache_size += content.len();
                        self.data_cache.insert(name, content);
                    }   
                }   
            } else { //use archive-reader
                let arc = Archive::open(archive_path);
                let mut content = vec![];
                if let Ok(_size) = arc.read_file(sub_path.to_str().unwrap(), &mut content) {
                    let name = archive_path.join(sub_path);
                    self.data_in_cache_size += content.len();
                    self.data_cache.insert(name, content);
                }
            }
        }
    }

    fn extract_from_data_cache(&mut self, archive_path: &Path, sub_path: &Path) {
        let mut content = vec![];
        if let Some(archive) = self.data_cache.get(archive_path) {
            let buf_read = Cursor::new(archive);
            let mut archive = zip::ZipArchive::new(buf_read).unwrap();
            if let Ok(mut file) = archive.by_name(sub_path.to_str().unwrap()) {                
                if let Ok(_size) = io::copy(&mut file, &mut content) {
                    //moved
                }
            }
        }
        if !content.is_empty() {
            let name = archive_path.join(sub_path);
            self.data_in_cache_size += content.len();
            self.data_cache.insert(name, content);
        }
    }

    fn get_filelist(archive_path: &Path) -> Result<Vec<String>, Box<dyn Error>> {
        let mut res: Vec<String> = Vec::new();

        if let Some(ex) = archive_path.extension() {
            if ex.eq_ignore_ascii_case("zip") { //use zip
                let f = File::open(archive_path).expect("unable to load file");
                let buf_read = BufReader::new(f);
                let archive = zip::ZipArchive::new(buf_read)?;
                for name in archive.file_names() {
                    res.push(name.to_string());
                }
            } else { //use archive-reader
                let mut arc = Archive::open(archive_path);
                res = arc
                    .block_size(1024*1024)
                    .list_file_names().expect("no files")
                    .collect::<Result<Vec<_>>>()?;
            }
        }
        Ok(res)
    }

    fn get_zip_filelist(archive: Vec<u8>) -> Vec<String> {
        let mut res: Vec<String> = Vec::new();
        let buf_read = Cursor::new(archive);
        let archive = zip::ZipArchive::new(buf_read).unwrap();
        for name in archive.file_names() {
            res.push(name.to_string());
        }
        res
    }

    fn add_filelist_to_directory(&mut self, list: Vec<String>, archive_path: PathBuf) {
        for path in list {
            let pb = PathBuf::from(&path);
            let mut entry_type = 
                if path.ends_with("/") {
                    EntryType::Dir                
                } else {
                    EntryType::File
                };

            if EntryType::File == entry_type {
                if let Some(ex) = pb.extension() {
                    for archex in ARCHIVE_EXTENSIONS {
                        if ex.eq_ignore_ascii_case(archex) {
                            entry_type = EntryType::Archive;
                            break;
                        }
                    }

                    if EntryType::Archive != entry_type {
                        for imagex in IMAGE_EXTENSIONS {
                            if ex.eq_ignore_ascii_case(imagex) {
                                entry_type = EntryType::Image;
                                break;
                            }
                        }
                    }
                }
            }

            let mut normals = Vec::new();
            for a in pb.components() {
                if let Component::Normal(os_str) = a {
                    normals.push(os_str);
                }
            }

            let mut directory_level = 1; //put this loop in components loop, but use a.next == none? to get level??
            let mut working_path = archive_path.to_path_buf();
            for part in &normals {
                if normals.len() == directory_level {
                    //we're at end
                    if !self.listings.contains_key(&working_path) {
                        self.listings.insert(working_path.clone(), Vec::new());
                    }
                    if let Some(list) = self.listings.get_mut(&working_path) {
                        list.push(Listing {
                            entry_type: entry_type.clone(),
                            display_name: part.display().to_string(),
                            file_path: archive_path.join(&path),
                            size: 0
                        });
                    }
                    //we're at end
                } else {
                    directory_level += 1;
                    let temp = working_path.join(part);
                    working_path = temp;
                }
            }
        }
    }

    fn list_dir(path: &Path) -> Result<Vec<Listing>, io::Error> {
        let paths = fs::read_dir(path)?;
        let mut new_listing: Vec<Listing> = Vec::new();
        for path in paths {
            match path {
                Err(_) => {}, //skips path enteries which err

                Ok(path) => {
                    let this_type: EntryType;
                    if path.metadata()?.is_dir() {
                        this_type = EntryType::Dir;
                    } else if path.metadata()?.is_file() {
                        this_type = EntryType::File;
                    } else {
                        this_type = EntryType::Link;
                    }
                    new_listing.push(Listing {
                        entry_type: this_type,
                        display_name: path.file_name().display().to_string(),
                        file_path: path.path(),
                        size: path.metadata()?.len()
                    });
                }
            }
        }
        new_listing = MyModel::set_archive_types(new_listing);
        new_listing = MyModel::set_image_types(new_listing);
        new_listing.sort();
        Ok(new_listing)
    }

    fn set_archive_types(mut list: Vec<Listing>) -> Vec<Listing>{
        for item in &mut list {
            if let Some(ex) = item.file_path.extension() {
                for archex in ARCHIVE_EXTENSIONS {
                    if ex.eq_ignore_ascii_case(archex) {
                        item.entry_type = EntryType::Archive;
                        break;
                    }
                }
            }
        }
        list
    }

    fn set_image_types(mut list: Vec<Listing>) -> Vec<Listing>{
        for item in &mut list {
            if let Some(ex) = item.file_path.extension() {
                for imagex in IMAGE_EXTENSIONS {
                    if ex.eq_ignore_ascii_case(imagex) {
                        item.entry_type = EntryType::Image;
                        break;
                    }
                }
            }
        }
        list
    }

    ///returns pathbuf and index for next image
    pub fn get_next_image(&self, cur: PathBuf) -> Option<(PathBuf, usize)> { //what kind of inefficiency is this function
        if let Some(current_listing) = self.listings.get(&self.cwd) {
            for (pos, listing) in current_listing.iter().enumerate() {
                if listing.file_path.eq(&cur) {
                    let remaning = &current_listing[pos+1..]; // from next
                    for (pos2, list_entry) in remaning.iter().enumerate() {
                        if let Some(ex) = list_entry.file_path.extension() {
                            for imgex in IMAGE_EXTENSIONS {
                                if ex.eq_ignore_ascii_case(imgex) {
                                    return Some((list_entry.file_path.clone(), pos+1 + pos2)); //remaning began from +1
                                }
                            }
                        }   
                    }
                }
            }
        }
        None
    }

    ///returns pathbuf and index for prev image
    pub fn get_prev_image(&self, cur: PathBuf) -> Option<(PathBuf, usize)> { //what kind of inefficiency is this function
        if let Some(current_listing) = self.listings.get(&self.cwd) {
            for (pos, listing) in current_listing.iter().enumerate() {
                if listing.file_path.eq(&cur) {
                    let preceding = &current_listing[0..pos]; // from -1 really, but including current because len() is 1 based
                    for n in (0..preceding.len()).rev() { //is there a usable next_back?
                        if let Some(ex) = preceding[n].file_path.extension() {
                            for imgex in IMAGE_EXTENSIONS {
                                if ex.eq_ignore_ascii_case(imgex) {
                                    return Some((preceding[n].file_path.clone(), pos - (preceding.len()-n) ));
                                }
                            }
                        }
                    }
                }
            }
        }
        None
    }

}
