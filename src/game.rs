use std::fmt::Debug;

use num_integer::Integer;

#[derive(Clone, Debug)]
pub struct Frame {
    pub width: u32,
    pub height: u32,
    pub cells: Vec<Cell>,
}

#[derive(Clone, PartialEq, Debug)]
pub enum Cell {
    Dead = 0,
    Alive = 1,
}

impl Frame {
    pub fn new(width: u32, height: u32) -> Self {
        let cells = (0..width * height).map(|_| Cell::Dead).collect();

        Frame {
            width,
            height,
            cells,
        }
    }

    pub fn step(&mut self) -> Vec<(u32, u32, Cell)> {
        let mut new_frame = self.clone();
        let mut diff = Vec::new();
        for (index, cell) in self.cells.iter().enumerate() {
            let (y, x) = (index as u32).div_rem(&self.width);
            match (self.count_neighbours(x, y), cell) {
                (n, Cell::Alive) if n != 2 && n != 3 => {
                    new_frame.set_cell(x, y, Cell::Dead);
                    diff.push((x, y, Cell::Dead));
                }
                (n, Cell::Dead) if n == 3 => {
                    new_frame.set_cell(x, y, Cell::Alive);
                    diff.push((x, y, Cell::Alive));
                }
                (_, _) => {}
            };
        }

        self.cells = new_frame.cells;
        diff
    }

    fn count_neighbours(&self, x: u32, y: u32) -> u8 {
        let mut neighbours = 0;
        for i in [self.width - 1, 0, 1] {
            for j in [self.height - 1, 0, 1] {
                if i == 0 && j == 0 {
                    continue;
                }
                let x = (x + i) % self.width;
                let y = (y + j) % self.height;
                neighbours += self.get_cell(x as u32, y as u32) as u8;
            }
        }

        neighbours
    }

    pub fn get_cell(&self, x: u32, y: u32) -> Cell {
        self.cells
            .get((y * self.width + x) as usize)
            .unwrap()
            .clone()
    }
    fn get_cell_mut(&mut self, x: u32, y: u32) -> &mut Cell {
        self.cells.get_mut((y * self.width + x) as usize).unwrap()
    }

    /// sets a cell and returns if the cell has changed
    pub fn set_cell(&mut self, x: u32, y: u32, cell: Cell) -> bool {
        let old_cell = self.get_cell_mut(x, y);
        if *old_cell != cell {
            *old_cell = cell;
            true
        } else {
            false
        }
    }
}
