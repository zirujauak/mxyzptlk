use super::Color;
use super::Style;

#[derive(Clone, Copy)]
pub struct CellStyle {
    mask: u8
}

impl CellStyle {
    pub fn new() -> CellStyle {
        CellStyle { mask: 0 }
    }

    pub fn set(&mut self, style: u8) {
        match style {
            0 => self.mask = 0,
            _ => self.mask = self.mask | style
        }
    }

    pub fn clear(&mut self, style: u8) {
        let mask = !(style as u8);
        self.mask = self.mask & mask;
    }

    pub fn is_style(&self, style: Style) -> bool {
        self.mask & style as u8 > 0 
    }
}

struct BufferCell {
    zchar: u16,
    // foreground, background)
    color: (Color, Color),
    style: CellStyle
}

impl BufferCell {
    pub fn new(zchar: u16, colors: (Color, Color), style: CellStyle) -> BufferCell {
        BufferCell { zchar, color: colors, style: style.clone() }
    }
}
pub struct Buffer {
    rows: u32,
    columns: u32,
    buffer: Vec<Vec<BufferCell>>
}

impl Buffer {
    pub fn new(rows: u32, columns: u32, colors: (Color, Color)) -> Buffer{
        let mut buffer: Vec<Vec<BufferCell>> = Vec::new();
        for i in 0..rows {
            let mut r = Vec::new();
            for j in 0..columns {
                r.push(BufferCell::new(' ' as u16, colors, CellStyle::new()));
            }
            buffer.push(r);
        }

        Buffer { rows, columns, buffer }
    }

    pub fn clear(&mut self, colors: (Color, Color), at: (u32,u32)) {
        self.buffer[at.0 as usize][at.1 as usize] = BufferCell::new(' ' as u16, colors, CellStyle::new());
    }

    pub fn print(&mut self, zchar: u16, colors: (Color, Color), style: &CellStyle, at: (u32, u32)) {
        self.buffer[at.0 as usize][at.1 as usize] = BufferCell::new(zchar, colors, style.clone());
    }

    pub fn scroll(&mut self, top: u32, colors: (Color, Color)) {
        // Remove the row at the top of the scroll window
        self.buffer.remove(top as usize - 1);
        let mut r = Vec::new();
        for i in 0..self.columns {
            r.push(BufferCell::new(' ' as u16, colors, CellStyle::new()))
        }
        self.buffer.push(r);
    }
}