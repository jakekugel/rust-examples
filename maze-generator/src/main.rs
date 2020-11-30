// This program creates a .pdf containing a generated maze.
//
// Author: Jake Kugel

use rand::prelude::*;
use rand::distributions::WeightedIndex;
use std::cmp::PartialEq;
use printpdf::*;
use std::fs::File;
use std::io::BufWriter;
use num_traits::pow;
use std::time::Instant;
use std::io::{self, Write};
use std::process;
use std::cmp;

// Represents a single cell in rectangular grid.  The 'edges' field contains
// array of booleans indicating a valid path starting at this cell and going
// to adjacent cell.
#[derive(PartialEq, Eq, Hash, PartialOrd, Ord, Copy, Clone, Debug)]
struct Cell {
    cell_type: u8,
    x: u32,
    y: u32,
    edges: [bool; 4],
    visited: bool,
    start_area: bool,
    finish_area: bool
}

impl Cell {
    fn draw_edge(&mut self, direction: &Direction) {
        self.edges[direction.to_usize()] = true;
    }

    fn mark_as_visited(&mut self) {
        self.visited = true;
    }

    fn has_edge(&self, direction: &Direction) -> bool {
        return self.edges[direction.to_usize()];
    }
}

struct Document {
    page_height_in: f64,
    page_width_in: f64,
    line_width_pt: f64,
    margin_in: f64
}

// Represents one of North, South, East, West
#[derive(PartialEq, Eq, Hash, PartialOrd, Ord, Copy, Clone)]
enum Direction {
    North,
    East,
    South,
    West
}

impl std::fmt::Display for Direction {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        let dir: &str = match self {
            Direction::North => "North",
            Direction::East => "East",
            Direction::South => "South",
            Direction::West => "West"
        };

        // This returns Result
        write!(f, "{}", dir)
    }
}

impl Direction {
    fn right(&self) -> Direction {
        match self {
            Direction::North => Direction::East,
            Direction::East => Direction::South,
            Direction::South => Direction::West,
            Direction::West => Direction::North
        }
    }

    fn left(&self) -> Direction {
        match self {
            Direction::North => Direction::West,
            Direction::West => Direction::South,
            Direction::South => Direction::East,
            Direction::East => Direction::North
        }
    }

    fn opposite(&self) -> Direction {
        match self {
            Direction::North => Direction::South,
            Direction::West => Direction::East,
            Direction::South => Direction::North,
            Direction::East => Direction::West
        }
    }

    fn to_usize(&self) -> usize {
        match self {
            Direction::North => 0,
            Direction::East => 1,
            Direction::South => 2,
            Direction::West => 3
        }
    }
}

// The strategy used to decide whether to continue forward, turn left, or
// turn right when drawing a path.
#[derive(Clone)]
struct MeanderStrategy {
    weight_north_south: u32,
    weight_east_west: u32,
    weight_forward: u32,
    weight_turn_left: u32,
    weight_turn_right: u32,
    weight_same_cell_type: u32
}

impl MeanderStrategy {
    fn new(i: u32, j: u32, k: u32, l: u32, m: u32, n: u32) -> MeanderStrategy {
        return MeanderStrategy {
            weight_north_south: i,
            weight_east_west: j,
            weight_forward: k,
            weight_turn_left: l,
            weight_turn_right: m,
            weight_same_cell_type: n
        };
    }

    // Get next direction based on meander strategy.  Direction is weighted
    // random choice based on struct weights.
    fn get_direction(&self, maze: &mut Maze, current: &Cell) -> Option<Direction> {
        let previous_direction = match maze.get_previous_direction(current) {
            Some(direction) => direction,
            None => Direction::North
        };

        let mut direction = previous_direction.clone();

        // Build a vector of valid directions and corresponding vector of weights
        let mut directions = Vec::<Direction>::new();
        let mut weights = Vec::<u32>::new();
        for _ in 0..4 {
            direction = direction.right();
            if maze.is_valid(current, &direction) {
                directions.push(direction.clone());
                weights.push(self.get_weight(maze, current, &direction, &previous_direction));
            }
        }

        // Make a weighted random choice
        if directions.len() > 0 {
            let dist = WeightedIndex::new(&weights).unwrap();


            return Some(directions.get(dist.sample(&mut maze.rng)).unwrap().clone());
        } else {
            None
        }
    }

    fn get_weight(&self, maze: &Maze, current: &Cell, direction: &Direction, previous_direction: &Direction) -> u32 {
        let mut weight: u32 = 0;

        // Because we called maze.is_valid() before get_weight(), we know
        // get_adjacent() will return a cell, and we can unwrap immediately.
        let next: Cell = maze.get_adjacent(current, direction).unwrap();

        if *direction == Direction::North || *direction == Direction::South {
            weight = weight + self.weight_north_south;
        } else {
            weight = weight + self.weight_east_west;
        }

        if current.cell_type == next.cell_type {
            weight = weight + self.weight_same_cell_type;
        }

        if direction == previous_direction {
            weight = weight + self.weight_forward;
        }

        if *direction == previous_direction.left() {
            weight = weight + self.weight_turn_left;
        }

        if *direction == previous_direction.right() {
            weight = weight + self.weight_turn_right;
        }

        return weight;
    }
}

// Represents a two-dimensional array of cells, also list of available strategies.
// The grid is a square of width and height equal to 'size'.  The lower-left
// corner is x=0, y=0, and the top-right corner is x=size-1, y=size-1.
struct Maze {
    cells: Vec<Vec<Cell>>,
    strategies: Vec<MeanderStrategy>,
    x_size: u32,
    y_size: u32,
    goal_reached: bool,
    start_finish_size: u32,
    finish_x: u32,
    finish_y: u32,
    start_x: u32,
    start_y: u32,
    rng: rand::rngs::ThreadRng
}

impl Maze {
    fn new(x_size: u32, y_size: u32, start_finish_size: u32) -> Maze {

        // Construct a column-major data structure so we can reference using
        // familiar [x][y] notation.
        let mut cells: Vec<Vec<Cell>> = Vec::new();
        for x in 0..x_size {
            let mut column_vec : Vec<Cell> = Vec::new();
            for y in 0..y_size {

                // cell_type - different cell types have different meander strategies
                let cell_type = if pow((x as f64 - (x_size as f64 / 2.0)) / x_size as f64, 2)
                                   + pow((y as f64 - (y_size as f64 / 2.0)) / x_size as f64, 2) < 0.15 {
                    0
                } else {
                    1
                };

                // start_area - if this cell is within the special start area where home icon shown
                let start_area = if x < start_finish_size && y < start_finish_size {
                    true
                } else {
                    false
                };

                // finish_area - if this cell is within the special finish area where star icon shown
                let finish_area = if (x >= (x_size - start_finish_size)) && (y >= (y_size - start_finish_size)) {
                    true
                } else {
                    false
                };

                let cell: Cell = Cell {
                    cell_type: cell_type,
                    edges: [false, false, false, false],
                    x: x,
                    y: y,
                    visited: false,
                    start_area: start_area,
                    finish_area: finish_area
                };

                column_vec.push(cell);
            }
            cells.push(column_vec);
        }

        let mut strategies: Vec<MeanderStrategy> = Vec::new();

        // Inside the circle, paths are generally long, straight east-west
        let strategy1 = MeanderStrategy::new(1, 100, 1, 1, 1, 1);

        // Outside the circle, paths move more randomly
        let strategy2 = MeanderStrategy::new(1, 1, 1, 1, 1, 1);

        strategies.push(strategy1);
        strategies.push(strategy2);

        let maze: Maze = Maze {
            cells: cells,
            strategies: strategies,
            x_size: x_size,
            y_size: y_size,
            goal_reached: false,
            start_finish_size: start_finish_size,
            start_x: 0,
            start_y: start_finish_size - 1,
            finish_x: 0,
            finish_y: 0,
            rng: thread_rng()
        };

        return maze;
    }

    fn meander(&mut self, current: &Cell) -> Option<Direction> {
        let strategy: MeanderStrategy = self.strategies.get(current.cell_type as usize).unwrap().clone();
        strategy.get_direction(self, current)
    }

    // Generate a directed, acyclic graph with the maze starting cell as the
    // root, and that visits every other cell in the square grid of cells.
    fn generate(&mut self) {
        let mut paths: Vec<Cell> = Vec::new();

        // Start with a single path
        paths.push(self.get_cell(self.start_x, self.start_y));

        while paths.len() > 0 {

            // Loop through the paths list and advance (or back-track) each.
            let mut index = 0;
            while index < paths.len() {
                let path = paths[index];

                let direction: Option<Direction> = self.meander(&path);
                match direction {
                    Some(direction) => {
                        // Five percent of the time, a path that is not at
                        // a dead end will split into two paths.
                        if self.rng.gen::<f64>() < 0.05 as f64 {
                            paths.push(self.get_cell(path.x, path.y));
                        }

                        let next = self.get_adjacent(&path, &direction).unwrap();

                        // Special handling if the next cell is in the finish area.
                        if next.finish_area {
                            self.draw_edge(&path, &direction);
                            self.mark_as_visited(&next);
                            self.finish_x = next.x;
                            self.finish_y = next.y;
                            self.goal_reached = true;
                        } else {
                            self.draw_edge(&path, &direction);
                            self.mark_as_visited(&next);
                            paths[index] = self.get_cell(next.x, next.y);
                        }

                        index = index + 1;

                    }
                    // No direction recommended - this tip is at dead end - need to back track
                    None => {
                        if path.x == self.start_x && path.y == self.start_y {
                            // If a path backtracks all the way to the beginning, remove
                            // it from paths vector.
                            paths.remove(index);
                        } else {
                            paths[index] = self.get_previous_cell(&path);
                            index = index + 1;
                        }
                    }
                }
            }
        }
    }

    fn draw_edge(&mut self, current: &Cell, direction: &Direction) {
        let current: &mut Cell = self.cells.get_mut(current.x as usize).unwrap().get_mut(current.y as usize).unwrap();
        current.draw_edge(direction);
    }

    fn mark_as_visited(&mut self, current: &Cell) {
        let current: &mut Cell = self.cells.get_mut(current.x as usize).unwrap().get_mut(current.y as usize).unwrap();
        current.mark_as_visited();
    }

    // Given a cell that has been visited already, return the direction
    // of the graph edge that arrives here.
    fn get_previous_direction(&self, cell: &Cell) -> Option<Direction> {
        let mut direction = Direction::North;

        // Loop through North, East, South, West and check
        for _ in 0..4 {
            match self.get_adjacent(cell, &direction) {
                Some(adjacent) => {
                    if adjacent.has_edge(&direction.opposite()) {
                        return Some(direction.opposite());
                    }
                },
                None => {}
            }

            direction = direction.right();
        }
        None
    }

    // Given a cell that has been visited already, follow the edge that
    // arrived here and return the previous cell.
    fn get_previous_cell(&self, cell: &Cell) -> Cell {
        let backward: &Direction = &self
            .get_previous_direction(&cell)
            .unwrap()
            .opposite();
        let previous = self.get_adjacent(&cell, backward).unwrap();

        self.get_cell(previous.x, previous.y)
    }

    // Given a cell and a direction, return the adjacent cell that is
    // arrived at by moving in the given direction.  If the direction would
    // fall outside of the bounds of the maze, None is returned.
    fn get_adjacent(&self, cell: &Cell, direction: &Direction) -> Option<Cell> {
        if *direction == Direction::North {
            if cell.y == self.y_size - 1 {
                None
            } else {
                Some(self.get_cell(cell.x, cell.y + 1))
            }
        } else if *direction == Direction::South {
            if cell.y == 0 {
                None
            } else {
                Some(self.get_cell(cell.x, cell.y - 1))
            }
        } else if *direction == Direction::East {
            if cell.x == self.x_size - 1 {
                None
            } else {
                Some(self.get_cell(cell.x + 1, cell.y))
            }
        } else { // west
            if cell.x == 0 {
                None
            } else {
                Some(self.get_cell(cell.x - 1, cell.y))
            }
        }
    }

    fn get_cell(&self, x: u32, y: u32) -> Cell {
        let cell_ref: &Cell = self.cells.get(x as usize).unwrap().get(y as usize).unwrap();
        (*cell_ref).clone()
    }

    // Returns true if the cell arrived at by moving in direction from current
    // cell is within maze bounds and hasn't been visited yet.
    fn is_valid(&self, current: &Cell, direction: &Direction) -> bool {
        match self.get_adjacent(current, direction) {
            // If the target cell is not within maze boundary, is_valid returns false
            None => false,
            Some(target_cell) => {
                if target_cell.visited || target_cell.start_area ||
                    target_cell.finish_area && self.goal_reached {
                        false
                    } else {
                        true
                    }
            }
        }
    }

    //https://docs.rs/printpdf/0.3.3/printpdf/types/pdf_layer/struct.PdfLayerReference.html#method.add_shape
    fn to_pdf(&self, doc: &Document, solution: bool, filename: &str) {
        let width_pt = Pt(doc.page_width_in * 72.0);
        let height_pt = Pt(doc.page_height_in * 72.0);
        let (pdf, page1, layer1) = PdfDocument::new("Maze", Mm::from(width_pt), Mm::from(height_pt), "Solution");
        let solution_layer = pdf.get_page(page1).get_layer(layer1);

        if solution {
            self.draw_solution(&solution_layer, doc);
        }

        let main_layer = pdf.get_page(page1).add_layer("Puzzle");
        self.draw_maze(&main_layer, doc);

        pdf.save(&mut BufWriter::new(File::create(filename).unwrap())).unwrap();
    }

    fn draw_maze(&self, layer: &PdfLayerReference, doc: &Document) {
        let fill_color = Color::Cmyk(Cmyk::new(1.0, 1.0, 1.0, 1.0, None));
        let outline_color = Color::Rgb(Rgb::new(0.0, 0.0, 0.0, None));
        let mut dash_pattern = LineDashPattern::default();
        dash_pattern.dash_1 = Some(20);

        layer.set_fill_color(fill_color);
        layer.set_outline_color(outline_color);
        layer.set_outline_thickness(doc.line_width_pt); // In points

        for x in -1..self.x_size as i32 + 1 {
            for y in -1..self.y_size as i32 + 1 {

                // Check if a horizontal line needed between cells (x, y) and (x, y + 1)
                let horizontal_needed =
                    if x == -1 || x == self.x_size as i32 { // left and right margin
                        false
                    } else if y == -1 || y == self.y_size as i32 - 1 { // top and bottom margin
                        true
                    } else if y == self.y_size as i32 {
                        false
                    } else {
                        let cell: Cell = self.get_cell(x as u32, y as u32);
                        let cell_above: Cell = self.get_cell(x as u32, y as u32 + 1);

                        if (cell_above.edges[2] || cell.edges[0]) ||  // There is a path between the two cells
                           (cell.start_area && cell_above.start_area) || // Two cells within start area
                           (cell.finish_area && cell_above.finish_area) { // Two cells within finish area
                            false
                        } else {
                            true
                        }
                    };

                if horizontal_needed {
                    let line: Line = get_line(x, y + 1, x + 1, y + 1, self.x_size as i32, self.y_size as i32, doc);
                    layer.add_shape(line);
                };

                // Check if a vertical line needed between cells (x, y) and (x + 1, y)
                let vertical_needed =
                    if y == -1 || y == self.y_size as i32 { // top and bottom margin
                        false
                    } else if x == -1 || x == self.x_size as i32 - 1 { // left and right margin
                        true
                    } else if x == self.x_size as i32 {
                        false
                    } else {
                        let cell: Cell = self.get_cell(x as u32, y as u32);
                        let cell_right: Cell = self.get_cell(x as u32 + 1, y as u32);

                        if (cell_right.edges[3] || cell.edges[1]) || // There is a path between two cells
                           (cell.start_area && cell_right.start_area) || // Two cells within start area
                           (cell.finish_area && cell_right.finish_area) { // Two cells within finish area
                            false
                        } else {
                            true
                        }
                    };

                if vertical_needed {
                    let line: Line = get_line(x + 1, y, x + 1, y + 1, self.x_size as i32, self.y_size as i32, doc);
                    layer.add_shape(line);
                };
            }
        }

        // Add house shape for starting area
        let home: Line = get_home_icon(self.start_finish_size, self.start_finish_size, 0, 0, self.x_size, self.y_size, doc);
        layer.add_shape(home);

        // Add star shape for finish area
        let star: Line = get_star_icon(
            self.start_finish_size, self.start_finish_size,
            self.x_size - self.start_finish_size, self.y_size - self.start_finish_size,
            self.x_size, self.y_size, doc);
        layer.add_shape(star);

    }

    fn draw_solution(&self, layer: &PdfLayerReference, doc: &Document) {
        let fill_color = Color::Cmyk(Cmyk::new(0.0, 0.0, 1.0, 0.0, None));
        let outline_color = Color::Rgb(Rgb::new(0.0, 0.0, 0.0, None));

        layer.set_fill_color(fill_color);
        layer.set_outline_color(outline_color);
        layer.set_outline_thickness(doc.line_width_pt); // In points

        let mut current_x = self.finish_x;
        let mut current_y = self.finish_y;

        let mut previous = self.get_previous_cell(&self.get_cell(current_x, current_y));

        while ! (current_x == self.start_x && current_y == self.start_y) {

            let x0 = cmp::min(current_x, previous.x);
            let y0 = cmp::min(current_y, previous.y);
            let x1 = cmp::max(current_x, previous.x) + 1;
            let y1 = cmp::max(current_y, previous.y) + 1;

            let rectangle = get_rectangle(x0, y0, x1, y1, self.x_size, self.y_size, doc);
            layer.add_shape(rectangle);

            current_x = previous.x;
            current_y = previous.y;

            if ! (current_x == self.start_x && current_y == self.start_y) {
                previous = self.get_previous_cell(&previous);
            }
        }

        // Highlight the starting region
        let rectangle = get_rectangle(0, 0, self.start_finish_size, self.start_finish_size, self.x_size, self.y_size, doc);
        layer.add_shape(rectangle);

        // Highlight the finish region
        let rectangle = get_rectangle(
                self.x_size - self.start_finish_size,
                self.y_size - self.start_finish_size,
                self.x_size,
                self.y_size,
                self.x_size, self.y_size,
                doc);
        layer.add_shape(rectangle);
    }
}

fn get_rectangle(x1: u32, y1: u32, x2: u32, y2: u32, x_size: u32, y_size: u32, doc: &Document) -> Line {
    let offset = Pt(doc.line_width_pt);

    let pdf_x0: Pt = transform_x(x1 as f64, x_size as i32, doc) + offset;
    let pdf_y0: Pt = transform_y(y1 as f64, y_size as i32, doc) + offset;
    let pdf_x1: Pt = transform_x(x2 as f64, x_size as i32, doc);
    let pdf_y1: Pt = transform_y(y2 as f64, y_size as i32, doc);

    let endpoints = vec![
        (Point::new(Mm::from(pdf_x0), Mm::from(pdf_y0)), false),
        (Point::new(Mm::from(pdf_x1), Mm::from(pdf_y0)), false),
        (Point::new(Mm::from(pdf_x1), Mm::from(pdf_y1)), false),
        (Point::new(Mm::from(pdf_x0), Mm::from(pdf_y1)), false),
        (Point::new(Mm::from(pdf_x0), Mm::from(pdf_y0)), false)];

    let line = Line {
        points: endpoints,
        is_closed: true,
        has_fill: true,
        has_stroke: false,
        is_clipping_path: false,
    };

    line
}

fn get_line(x1: i32, y1: i32, x2: i32, y2: i32, x_size: i32, y_size: i32, doc: &Document) -> Line {
    let pdf_x1: Pt = transform_x(x1 as f64, x_size, doc);
    let pdf_x2: Pt = transform_x(x2 as f64, x_size, doc);
    let pdf_y1: Pt = transform_y(y1 as f64, y_size, doc);
    let pdf_y2: Pt = transform_y(y2 as f64, y_size, doc);

    let horizontal = if x1 == x2 { false } else { true };

    // The line endpoints need to be offset slightly to compensate for the
    // line width.
    let (x1_offset, x2_offset, y1_offset, y2_offset) = if horizontal {
        (
            Pt(0.0),
            Pt(doc.line_width_pt),
            Pt(doc.line_width_pt / 2.0),
            Pt(doc.line_width_pt / 2.0))
    } else {
        (
            Pt(doc.line_width_pt / 2.0),
            Pt(doc.line_width_pt / 2.0),
            Pt(0.0),
            Pt(doc.line_width_pt))
    };

    let endpoints = vec![(Point::new(Mm::from(pdf_x1 + x1_offset), Mm::from(pdf_y1 + y1_offset)), false),
                       (Point::new(Mm::from(pdf_x2 + x2_offset), Mm::from(pdf_y2 + y2_offset)), false)];
    Line {
        points: endpoints,
        is_closed: false,
        has_fill: false,
        has_stroke: true,
        is_clipping_path: false,
    }
}

fn get_home_icon(cell_width: u32, cell_height: u32, cell_x: u32, cell_y: u32, x_size: u32, y_size: u32, doc: &Document) -> Line {
    // Points for home icon within scale (0,0) - (14, 14)
    let points: [(u8, u8); 13] = [
        (3, 2),
        (3, 8),
        (7, 12),
        (8, 11),
        (8, 12),
        (10, 12),
        (10, 9),
        (11, 8),
        (11, 2),
        (8, 2),
        (8, 6),
        (6, 6),
        (6, 2)
    ];

    // Scale to x, y coordinates used by cells
    let mut pdf_points: Vec<(Point, bool)> = Vec::new();

    for (i, j) in points.iter() {
        let x = *i as f64 / 14.0 * cell_width as f64 + cell_x as f64;
        let y = *j as f64 / 14.0 * cell_height as f64 + cell_y as f64;

        let pdf_x = transform_x(x, x_size as i32, doc);
        let pdf_y = transform_y(y, y_size as i32, doc);

        pdf_points.push((Point::new(Mm::from(pdf_x), Mm::from(pdf_y)), false));
    }

    Line {
        points: pdf_points,
        is_closed: true,
        has_fill: true,
        has_stroke: false,
        is_clipping_path: false,
    }
}

fn get_star_icon(cell_width: u32, cell_height: u32, cell_x: u32, cell_y: u32, x_size: u32, y_size: u32, doc: &Document) -> Line {

    // Radius of the points of the star
    let r1 = 5.0;

    // Radius to the inside angle between points
    let r2 = r1 * (360.0 / 5.0 as f64).to_radians().cos() / (360.0 / 10.0 as f64).to_radians().cos();

    // Scale to x, y coordinates used by cells
    let mut pdf_points: Vec<(Point, bool)> = Vec::new();

    for star_point in 0..5 {
        let radians = (star_point as f64 * 360.0 / 5.0).to_radians();
        let inner_offset = (360.0 / 10.0 as f64).to_radians();

        // First find coordinates for 5-pointed star centered in arbitrary 14x14 region
        let x1 = 7.0 + r1 * radians.sin();
        let y1 = 7.0 + r1 * radians.cos();
        let x2 = 7.0 + r2 * (radians + inner_offset).sin();
        let y2 = 7.0 + r2 * (radians + inner_offset).cos();

        // Map coordinates to the coordinate system used by maze, placing star into the
        // designated goal area.
        let x1 = x1 / 14.0 * cell_width as f64 + cell_x as f64;
        let y1 = y1 / 14.0 * cell_height as f64 + cell_y as f64;
        let x2 = x2 / 14.0 * cell_width as f64 + cell_x as f64;
        let y2 = y2 / 14.0 * cell_height as f64 + cell_y as f64;

        // Map to PDF coordinates
        let pdf_x1 = transform_x(x1, x_size as i32, doc);
        let pdf_y1 = transform_y(y1, y_size as i32, doc);
        let pdf_x2 = transform_x(x2, x_size as i32, doc);
        let pdf_y2 = transform_y(y2, y_size as i32, doc);

        pdf_points.push((Point::new(Mm::from(pdf_x1), Mm::from(pdf_y1)), false));
        pdf_points.push((Point::new(Mm::from(pdf_x2), Mm::from(pdf_y2)), false));
    }

    Line {
        points: pdf_points,
        is_closed: true,
        has_fill: true,
        has_stroke: false,
        is_clipping_path: false,
    }
}

fn transform_x(x: f64, x_size: i32, doc: &Document) -> Pt {
    let points_per_inch: f64 = 72.0;

    Pt(((x / x_size as f64) * (doc.page_width_in - 2.0 * doc.margin_in) + doc.margin_in) * points_per_inch)
}

fn transform_y(y: f64, y_size: i32, doc: &Document) -> Pt {
    let points_per_inch: f64 = 72.0;

    Pt(((y / y_size as f64) * (doc.page_height_in - 2.0 * doc.margin_in) + doc.margin_in) * points_per_inch)
}

fn get_user_input() -> (f64, f64, f64, f64) {
    let size = get_text_input("Enter maze cell size (micro, small, medium, large) [medium]: ");
    let (cell_size, line_width_pt) = match size.as_str() {
       "micro" => (1.0 / 16.0, 0.0),
       "small" => (0.125, 1.0),
       "medium" => (0.25, 2.0),
       "large" => (0.5, 3.0),
       "" => (0.25, 2.0),
       _ => {
         println!("Must enter 'micro', 'small', 'medium', or 'large'.");
         process::exit(1);
       }
    };

    let page_size = get_text_input("Enter paper size (letter, custom) [letter]: ");
    let (page_width_in, page_height_in) = match page_size.as_str() {
       "letter" => (8.5, 11.0),
       "custom" => {
            let page_width_in = get_float_input("Enter page width in inches (1..1000): ");
            if page_width_in < 1.0 || page_width_in > 1000.0 {
                println!("Must enter value between 1 and 1000.");
                process::exit(1);
            }

            let page_height_in = get_float_input("Enter page height in inches (1..1000): ");
            if page_height_in < 1.0 || page_height_in > 1000.0 {
                println!("Must enter value between 1 and 1000.");
                process::exit(1);
            }

            (page_width_in, page_height_in)
       },
       "" => (8.5, 11.0),
       _ => {
         println!("Must enter 'letter' or 'custom'.");
         process::exit(1);
       }
    };

    (cell_size, line_width_pt, page_width_in, page_height_in)
}

fn get_text_input(prompt: &str) -> String {
    print!("{}", prompt);
    io::stdout().flush().unwrap();

    let mut response = String::new();

    io::stdin()
        .read_line(&mut response)
        .expect("Failed to read line");
    let response = response.trim().to_string().to_lowercase();
    response
}


fn get_float_input(prompt: &str) -> f64 {
    print!("{}", prompt);
    io::stdout().flush().unwrap();

    let mut response = String::new();

    io::stdin()
        .read_line(&mut response)
        .expect("Failed to read line");
    let response = response.trim().to_string().parse::<f64>();
    let response = match response {
        Ok(value) => value,
        Err(_err) => {
            println!("Must enter a numeric value.");
            process::exit(1);
        }
    };
    response
}


fn main() {
    // Prompt user for inputs
    let (cell_size, line_width_pt, page_width_in, page_height_in) = get_user_input();

    // Determine page margin based on page size
    let mut margin_in: f64 = 0.5;
    if page_width_in > 11.0 && page_height_in > 17.0 {
        margin_in = 1.0;
    }

    let x_size = ((page_width_in - margin_in * 2.0) / (cell_size)) as u32;
    let y_size = ((page_height_in - margin_in * 2.0) / (cell_size)) as u32;
    let mut maze: Maze = Maze::new(x_size, y_size, 3);

    let start = Instant::now();
    maze.generate();

    println!("Generated maze in {} milliseconds.", start.elapsed().as_millis());

    let doc: Document = Document {
        page_height_in,
        page_width_in,
        line_width_pt,
        margin_in
    };

    maze.to_pdf(&doc, false, "maze.pdf");
    maze.to_pdf(&doc, true, "solution.pdf");

    println!("Generated PDFs in {} milliseconds.", start.elapsed().as_millis());

}
