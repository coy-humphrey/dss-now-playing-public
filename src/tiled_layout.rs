extern crate sdl2;

use sdl2::pixels::Color;
use sdl2::rect::Point;
use sdl2::rect::Rect;
use sdl2::render::Canvas;
use sdl2::video::Window;

use crate::async_resource_manager::AsyncResourceManager;
use crate::json_parser::{RowInfo, TileInfo};

const TILE_COLOR: Color = Color::BLUE;

struct Tile {
    tile_info: TileInfo,
}

impl Tile {
    fn new(tile_info: TileInfo) -> Self {
        Self { tile_info }
    }

    fn draw(
        &self,
        canvas: &mut Canvas<Window>,
        texture_manager: &mut AsyncResourceManager,
        pos: Point,
        width: u32,
        height: u32,
        selected: bool,
    ) {
        let (width, height) = if selected {
            (width + width / 10, height + height / 10)
        } else {
            (width, height)
        };

        if selected {
            // +2 on width and height allows for a 1px wide outer layer
            let outer_rect = Rect::from_center(pos, width + 2, height + 2);
            canvas.set_draw_color(Color::WHITE);
            canvas.fill_rect(outer_rect).unwrap();
        }

        let rect = Rect::from_center(pos, width, height);
        let texture = texture_manager.get_image_from_url(&self.tile_info.img_url);
        if let Some(texture) = texture {
            canvas.copy(&texture, None, rect).unwrap();
        } else {
            canvas.set_draw_color(TILE_COLOR);
            canvas.fill_rect(rect).unwrap();
        }
    }
}

struct TileRow {
    window_start: usize,
    window_size: usize,
    title: String,
    tiles: Vec<Tile>,
}

impl TileRow {
    fn new_with_row_info(window_size: usize, row_info: RowInfo) -> Self {
        let mut tiles = Vec::new();
        for tile in row_info.tiles {
            tiles.push(Tile::new(tile));
        }

        Self {
            window_start: 0,
            window_size,
            title: row_info.title,
            tiles,
        }
    }

    fn rotate(&mut self, right: bool) {
        if self.tiles.is_empty() {
            return;
        }

        if right {
            self.window_start += 1;
            if self.window_start >= self.tiles.len() {
                self.window_start = 0;
            }
        // left
        } else if self.window_start == 0 {
            self.window_start = self.tiles.len() - 1;
        } else {
            self.window_start -= 1;
        }
    }

    fn draw(
        &self,
        canvas: &mut Canvas<Window>,
        texture_manager: &mut AsyncResourceManager,
        left_x: i32,
        center_y: i32,
        element_width: u32,
        element_height: u32,
        padding: (u32, u32),
        // If this row is selected, the usize will be the relative position of the selected tile
        // from [0, window_size)
        selected: Option<usize>,
    ) {
        let (w_padding, h_padding) = padding;
        // Display category title
        let text_y = center_y - element_height as i32 / 2 - h_padding as i32 / 2;
        let (texture, (text_width, text_height)) =
            texture_manager.get_text_texture_and_size(&self.title);
        let text_rect = Rect::from_center(
            Point::new(left_x + text_width as i32 / 2, text_y),
            text_width,
            text_height,
        );
        canvas.copy(&texture, None, text_rect).unwrap();
        drop(texture);

        if self.tiles.is_empty() {
            return;
        }

        let mut iter = self.tiles.iter().cycle();
        for _ in 0..self.window_start {
            iter.next();
        }
        let tile_y = center_y;

        let mut tile_x = left_x + w_padding as i32 / 2 + element_width as i32 / 2;
        for (i, tile) in iter.take(self.window_size).enumerate() {
            tile.draw(
                canvas,
                texture_manager,
                Point::new(tile_x, tile_y),
                element_width,
                element_height,
                selected.is_some() && (i == selected.unwrap()),
            );
            tile_x += element_width as i32 + w_padding as i32;
        }
    }
}

#[derive(Clone, Copy, PartialEq)]
pub enum Direction {
    Up,
    Down,
    Left,
    Right,
}

pub struct TiledLayout {
    row_col: (usize, usize),
    window_start: usize,
    vert_window_size: usize,
    hori_window_size: usize,
    left_x: i32,
    upper_y: i32,
    tile_rows: Vec<TileRow>,
}

impl TiledLayout {
    pub fn new_with_row_infos(row_infos: Vec<RowInfo>) -> Self {
        let hori_window_size = 4;
        let mut tile_rows = Vec::new();
        for info in row_infos {
            tile_rows.push(TileRow::new_with_row_info(hori_window_size, info));
        }

        Self {
            row_col: (0, 0),
            window_start: 0,
            vert_window_size: 4,
            hori_window_size,
            left_x: 0,
            upper_y: 0,
            tile_rows,
        }
    }

    fn rotate(&mut self, down: bool) {
        if down {
            self.window_start += 1;
            if self.window_start >= self.tile_rows.len() {
                self.window_start = 0;
            }
        // Up
        } else if self.window_start == 0 {
            self.window_start = self.tile_rows.len() - 1;
        } else {
            self.window_start -= 1;
        }
    }

    pub fn handle_direction(&mut self, direction: Direction) {
        match direction {
            Direction::Left => {
                if self.row_col.1 == 0 {
                    // row_col.0 is relative to window_start
                    // so row_col.0 + window_start gives us the selected row
                    // mod total number of rows in case we wrap around
                    let idx = (self.window_start + self.row_col.0) % self.tile_rows.len();
                    self.tile_rows[idx].rotate(false);
                } else {
                    self.row_col.1 -= 1;
                }
            }
            Direction::Right => {
                if self.row_col.1 == self.hori_window_size - 1 {
                    let idx = (self.window_start + self.row_col.0) % self.tile_rows.len();
                    self.tile_rows[idx].rotate(true);
                } else {
                    self.row_col.1 += 1;
                }
            }
            Direction::Up => {
                if self.row_col.0 == 0 {
                    self.rotate(false);
                } else {
                    self.row_col.0 -= 1;
                }
            }
            Direction::Down => {
                if self.row_col.0 == self.vert_window_size - 1 {
                    self.rotate(true);
                } else {
                    self.row_col.0 += 1;
                }
            }
        }
    }

    pub fn draw(
        &self,
        canvas: &mut Canvas<Window>,
        texture_manager: &mut AsyncResourceManager,
        width: u32,
        height: u32,
    ) {
        if self.tile_rows.is_empty() {
            return;
        }
        let mut iter = self.tile_rows.iter().cycle();
        for _ in 0..self.window_start {
            iter.next();
        }
        let (w_padding, h_padding) = (width / 20, height / 20);
        let element_height = height / self.vert_window_size as u32 - h_padding;
        let element_width = width / self.hori_window_size as u32 - w_padding;
        let mut center_y = self.upper_y + h_padding as i32 + element_height as i32 / 2;
        for (i, tilerow) in iter.take(self.vert_window_size).enumerate() {
            let selected = if i == self.row_col.0 {
                Some(self.row_col.1)
            } else {
                None
            };
            tilerow.draw(
                canvas,
                texture_manager,
                self.left_x,
                center_y,
                element_width as u32,
                element_height as u32,
                (w_padding, h_padding),
                selected,
            );
            center_y += element_height as i32 + h_padding as i32;
        }
    }
}
