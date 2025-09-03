# FqView
FqView is an application I use to learn Rust programming. The main purpose is to quickly be able to view images by pre-loading them into video memory. It can browse into archives to load images. It uses Speedy2D to upload images as OpenGL textures.

![fq2](https://github.com/user-attachments/assets/59e835b9-4e77-4b14-9e72-d7122203502f)

## File support
FqView uses image crate, zip crate, libarchive and libheif to support these image and archive types:
```
Images: bmp, dds, ff, gif, hdr, ico, jpg, jpeg, exr, png, pnm, qoi, tga, tif, tiff, webp, heic and heif

Archives: iso, zip, 7z, cab, rar, xar, lzh, lha, gz, bz2 and zst
```
For zip-type archives it can browse into archives, in archives, in archives, etc. without using temporary files - but limited by available memory.

![fq1](https://github.com/user-attachments/assets/88f75fab-2a23-4ac0-bb23-fdf837855f32)

## Controls
Currently these controls are configured:
* Next/prev image - Mouse wheel, PageDown/PageUp
* Zoom in/out - Pause/ScrollLock
* Zoom 1:1 - / or Insert
* Zoom fit to window - * or Delete
<br><br>Move part image displayed using arrow keys or mouse dragging.  
Holding Ctrl/Shift and an arrow key makes the scrolling faster/slower.  
Right click in browser view goes up a directory level.

## TODO
As a learning project there are plenty to improve upon. In no particular order:
* Support opening files from commandline
* Make it possible to unload images and data
* Change caching to preload more images
* Make memory usage data concur with external tools
* Save view settings for images on program exit
* Clean up unwrap()'s
* Improve code clarity
* Add scrollbars to imageview
* Make browser columns resizable
* Remember last selected file in directory
* Support sorting in fileview
* Display filesizes inside supported archives
* Auto-scroll statusbar messages
* Add [index/ ..] of browsable images to window label
* Document functions
* Use Rust lifetime specifiers
* HiDPI support / resize viewport on fltk global zoom
* Support other OS's than Linux
* Force image load with Ctrl
* Force archive load with Alt
  
