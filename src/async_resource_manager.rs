use bytes::Bytes;
use std::{collections::HashMap, rc::Rc, time::Duration};

use sdl2::{
    image::LoadTexture,
    pixels::Color,
    render::{Texture, TextureCreator},
    ttf::Font,
    video::WindowContext,
};
use tokio::sync::{
    mpsc::{self, error::TrySendError},
    oneshot::{self, error::TryRecvError},
};

#[derive(Debug)]
pub struct DownloadResponse {
    pub bytes: Bytes,
}

// Provides a url for the download loop to download,
// and a channel to send the downloaded bytes, or None if there's an error.
// Future: Probably better to use a result here in a real application,
// but for this POC we don't care about the error specifics
pub struct DownloadRequest {
    pub url: String,
    pub response_channel: oneshot::Sender<Option<DownloadResponse>>,
}

// Meant to be spawned off as a "parallel" async task
// Infinitely loops reading and handling download requests from the receiver
// If bounded is true, will handle requests one at a time, otherwise spawns a new task for each request.
// If slow is true, adds a 2 second pause for each request to (poorly) simulate a slow connection
pub async fn download_loop(mut rx: mpsc::Receiver<DownloadRequest>, slow: bool, bounded: bool) {
    while let Some(req) = rx.recv().await {
        if bounded {
            handle_request(req, slow).await;
        } else {
            tokio::spawn(handle_request(req, slow));
        }
    }
}

async fn handle_request(req: DownloadRequest, slow: bool) {
    let get_resp = reqwest::get(req.url).await;
    let download_resp = match get_resp {
        Ok(data) => match data.bytes().await {
            Ok(bytes) => Some(DownloadResponse { bytes }),
            // For this POC, just print errors we encounter
            // Futre improvement might be to change response to a result instead of Option
            // and let requester handle the error appropriately
            Err(e) => {
                println!("{}", e);
                None
            }
        },
        Err(e) => {
            println!("{}", e);
            None
        }
    };
    if slow {
        tokio::time::sleep(Duration::from_millis(2000)).await;
    }
    if req.response_channel.send(download_resp).is_err() {
        println!("Response channel closed unexpectedly");
    }
}

pub struct AsyncResourceManager<'l> {
    // Textures are only valid while TextureCreator lives
    // Lifetime of this struct and all textures should match texture creator
    texture_creator: &'l TextureCreator<WindowContext>,
    cache: HashMap<String, Rc<Texture<'l>>>,
    font_cache: HashMap<String, (Rc<Texture<'l>>, (u32, u32))>,
    in_progress: HashMap<String, oneshot::Receiver<Option<DownloadResponse>>>,
    default_font: Font<'l, 'l>,
    tx: mpsc::Sender<DownloadRequest>,
}

impl<'l> AsyncResourceManager<'l> {
    pub fn new(
        texture_creator: &'l TextureCreator<WindowContext>,
        tx: mpsc::Sender<DownloadRequest>,
        font: Font<'l, 'l>,
    ) -> Self {
        Self {
            texture_creator,
            cache: HashMap::new(),
            font_cache: HashMap::new(),
            in_progress: HashMap::new(),
            tx,
            default_font: font,
        }
    }

    pub fn get_text_texture_and_size(&mut self, text: &str) -> (Rc<Texture>, (u32, u32)) {
        if self.font_cache.contains_key(text) {
            self.font_cache.get(text).unwrap().clone()
        } else {
            let surface = self
                .default_font
                .render(text)
                .blended(Color::RGBA(255, 255, 255, 255))
                .unwrap();
            let texture = Rc::new(
                self.texture_creator
                    .create_texture_from_surface(&surface)
                    .unwrap(),
            );
            let font_size = self.default_font.size_of(text).unwrap();
            self.font_cache
                .insert(text.to_string(), (texture.clone(), font_size));
            (texture, font_size)
        }
    }

    pub fn get_image_from_url(&mut self, url: &str) -> Option<Rc<Texture>> {
        if self.cache.contains_key(url) {
            // Cloning an Rc is relatively cheap because we're just cloning the pointer.
            // We do NOT clone the texture it points to, which could be expensive.
            // Using an Rc instead of a reference gives us a bit more flexibility with the borrow checker
            self.cache.get(url).cloned()
        } else {
            // If a cached copy doesn't exist, and if there's not an in-progress request for this url,
            // issue a request to the download loop to download this image
            if !self.in_progress.contains_key(url) {
                let (resp_tx, resp_rx) = oneshot::channel();
                let msg = DownloadRequest {
                    url: url.to_string(),
                    response_channel: resp_tx,
                };
                match self.tx.try_send(msg) {
                    Ok(_) => {
                        self.in_progress.insert(url.to_string(), resp_rx);
                    }
                    // If other side is closed, we cannot recover
                    Err(TrySendError::Closed(_)) => panic!("Downloader closed unexpectedly"),
                    // All other errors can be ignored for this POC
                    Err(_) => (),
                }
            }
            None
        }
    }

    pub fn process_pending(&mut self) {
        // Can't easily modify a map while iterating through it,
        // so maintain list of what needs to be removed after the loop
        let mut remove_set = Vec::new();
        for (key, rx) in self.in_progress.iter_mut() {
            // try_recv instantly returns with either a valid value, or an error
            // There's no blocking and no need to await or yield control of the thread
            match rx.try_recv() {
                // Take image, convert to texture and add to cache
                Ok(val) => {
                    remove_set.push(key.clone());
                    if let Some(resp) = val {
                        let texture = self.texture_creator.load_texture_bytes(&resp.bytes);
                        if let Ok(texture) = texture {
                            self.cache.insert(key.clone(), Rc::new(texture));
                        }
                    }
                }
                // If other side closed unexpectedly, we can remove it and try again later
                Err(TryRecvError::Closed) => remove_set.push(key.clone()),
                // Result not ready yet, we'll try again next tick
                Err(TryRecvError::Empty) => (),
            }
        }

        for key in remove_set {
            self.in_progress.remove(&key);
        }
    }
}
