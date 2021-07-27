extern crate sdl2;

use dss_now_playing::async_resource_manager::DownloadRequest;
use sdl2::event::Event;
use sdl2::keyboard::Keycode;
use sdl2::pixels::Color;
use tokio::time::MissedTickBehavior;

use std::time::Duration;
use tokio::sync::mpsc;
use tokio::time;

// use dss_now_playing::json_parser::*;
use dss_now_playing::async_resource_manager::download_loop;
use dss_now_playing::async_resource_manager::AsyncResourceManager;
use dss_now_playing::json_parser::*;
use dss_now_playing::tiled_layout::*;

use clap::{AppSettings, Clap};

const BACKGROUND_COLOR: Color = Color::BLACK;

// Hard code width and height for this POC.
// In the future, would be best to make this dynamic
const WIDTH: u32 = 1280;
const HEIGHT: u32 = 720;

/// A proof of concept tiled display written in Rust
#[derive(Clap)]
#[clap(version = "0.1", author = "Coy Humphrey <coy@coyhumphrey.com>")]
#[clap(setting = AppSettings::ColoredHelp)]
struct Opts {
    /// Slows image downloads to show off asynchronous behavior
    #[clap(short, long)]
    slow: bool,
    /// Limits to a single active download
    #[clap(short, long)]
    bounded: bool,
    /// Use multiple threads
    #[clap(short, long)]
    threaded: bool,
    /// TTF font file for displaying text
    font_path: String,
}

async fn event_loop(row_infos: Vec<RowInfo>, tx: mpsc::Sender<DownloadRequest>, font_path: String) {
    let sdl_context = sdl2::init().unwrap();
    let video_subsystem = sdl_context.video().unwrap();

    let window = video_subsystem
        .window("DSS Now Playing", WIDTH, HEIGHT)
        .position_centered()
        .opengl()
        .build()
        .unwrap();

    let mut canvas = window.into_canvas().build().unwrap();
    let texture_creator = canvas.texture_creator();

    let ttf_context = sdl2::ttf::init().unwrap();

    // We use HEIGHT / 36 because it's half the size of HEIGHT / 18 used as height padding in the layout.
    // Future: Font sizing should be requested by tiled layout so internal details of the laytout don't
    // need to be known by anything else.
    let font = ttf_context
        .load_font(font_path, HEIGHT as u16 / 36)
        .unwrap();
    let mut texture_manager = AsyncResourceManager::new(&texture_creator, tx, font);

    let mut event_pump = sdl_context.event_pump().unwrap();

    let mut tile_set = TiledLayout::new_with_row_infos(row_infos);

    // Arbitary target of 20 Frames per second
    let mut interval = time::interval(Duration::from_millis(1000 / 20));
    // When a tick is missed, treat it as Delayed. It will continue with the same interval
    // from the point it gets picked up after the delay.
    // This changes from the default Burst mode, where future ticks are shortened
    // to make up for a lost tick.
    interval.set_missed_tick_behavior(MissedTickBehavior::Delay);

    'outer: loop {
        // Handle new events
        for event in event_pump.poll_iter() {
            match event {
                // On quit or escape, we break this loop and the function returns
                // Tokio should be blocking on this function, so this will also end the process
                Event::Quit { .. }
                | Event::KeyDown {
                    keycode: Some(Keycode::Escape),
                    ..
                } => break 'outer,
                Event::KeyDown {
                    keycode: Some(key), ..
                } => {
                    let maybe_direction = match key {
                        Keycode::Left => Some(Direction::Left),
                        Keycode::Right => Some(Direction::Right),
                        Keycode::Up => Some(Direction::Up),
                        Keycode::Down => Some(Direction::Down),
                        _ => None,
                    };

                    if let Some(dir) = maybe_direction {
                        tile_set.handle_direction(dir);
                    }
                }
                _ => {}
            }
        }
        // Update display
        // Future: Could look into logic to detect "dirty" tiles / screen.
        // Only update display if at least 1 element has changed, and possibly
        // only draw elements that have changed (would require removing clear())
        canvas.set_draw_color(BACKGROUND_COLOR);
        canvas.clear();

        tile_set.draw(&mut canvas, &mut texture_manager, WIDTH, HEIGHT);
        canvas.present();

        // Handle completed download requests
        texture_manager.process_pending();

        // Wait for next frame
        interval.tick().await;
    }
}

pub fn main() {
    let opts: Opts = Opts::parse();

    // Parse all json upfront.
    // We need at least the main file parsed before we can display anything useful.
    // Ideally we would parse refs only as needed, but for this POC we simplify
    // by parsing refs at the same time.
    let mut json_parser = JsonParser::new();
    let row_infos = json_parser.parse_all_rows();

    let rt = if opts.threaded {
        tokio::runtime::Builder::new_multi_thread()
    } else {
        tokio::runtime::Builder::new_current_thread()
    }
    .enable_time()
    .enable_io()
    .build()
    .unwrap();
    // Channel to allow event loop to request image downloads
    let (tx, rx) = mpsc::channel(16);
    // Infinite loop that processes download requests from main event loop
    rt.spawn(download_loop(rx, opts.slow, opts.bounded));
    // Infinite loop that updates display and handles user input
    rt.block_on(event_loop(row_infos, tx, opts.font_path));
}
