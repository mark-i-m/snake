
extern crate rand;
extern crate cursive;

use std::cmp::min;
use std::collections::{LinkedList, HashSet};

use cursive::{Cursive, Printer};
use cursive::views::{BoxView, Canvas, TextView, Dialog};
use cursive::event::{Key, Event, EventResult};
use cursive::theme::ColorStyle;

#[derive(Debug, Clone, Copy)]
enum Direction {
    North,
    South,
    East,
    West,
}

#[derive(Debug)]
struct State {
    size: (usize, usize),
    direction: Direction,
    snake: LinkedList<(usize, usize)>,
    food: HashSet<(usize, usize)>,

    prev_loc: LinkedList<(usize, usize)>,
}

fn main() {
    let mut siv = Cursive::new();
    siv.set_fps(2);

    let canvas = BoxView::with_full_screen(
        Canvas::new(State::new(siv.screen_size().pair()))
            .with_draw(|printer, state| state.draw(printer))
            .with_on_event(|event, state| match event {
                Event::Char('q') => EventResult::Ignored,
                /*
                Event::Key(Key::Up) => state.step(Some(Direction::North)),
                Event::Key(Key::Down) => state.step(Some(Direction::South)),
                Event::Key(Key::Left) => state.step(Some(Direction::West)),
                Event::Key(Key::Right) => state.step(Some(Direction::East)),
                */
                _ => state.step(None),
            }),
    );

    siv.add_fullscreen_layer(canvas);

    siv.add_global_callback('q', |s| s.quit());

    siv.run();
}

fn distance(
    (sizex, sizey): (usize, usize),
    (ax, ay): (usize, usize),
    (bx, by): (usize, usize),
) -> usize {
    let x_dist = if ax > bx { ax - bx } else { bx - ax };
    let y_dist = if ay > by { ay - by } else { by - ay };

    min(x_dist, sizex - x_dist) + min(y_dist, sizey - y_dist)
}

impl State {
    pub fn new(size: (usize, usize)) -> State {
        let mut snake = LinkedList::new();
        snake.push_back((0, 0));

        let mut state = State {
            size: size,
            direction: Direction::East,
            snake: snake,
            food: HashSet::new(),

            prev_loc: LinkedList::new(),
        };

        state.add_random_food();

        state
    }

    // Higher rank is worse
    fn rank(&self, direction: Direction) -> isize {
        let loc = self.next_loc(direction);

        let food_rank: isize = self.food
            .iter()
            .map(|&food| distance(self.size, loc, food) as isize)
            .min()
            .unwrap();

        let body_size = self.snake.len();
        let body_rank: isize = self.snake
            .iter()
            .take(body_size - 1)
            .map(|&body| distance(self.size, loc, body) as isize)
            .sum();

        const HISTORY: usize = 500;
        let been_there = if self.prev_loc.iter().take(HISTORY).any(|&prev| prev == loc) {
            1000
        } else {
            0
        };

        let is_loss = if self.snake.iter().any(|&body| body == loc) {
            100000
        } else {
            0
        };

        let is_eat = if self.food.iter().any(|&food| food == loc) {
            10000
        } else {
            0
        };

        let is_dead_end = if self.snake
            .iter()
            .take(body_size - 1)
            .map(|&body| distance(self.size, loc, body) as isize)
            .filter(|&d| d < 5)
            .count() > 1
        {
            10000
        } else {
            0
        };

        is_loss + food_rank + been_there - body_rank - is_eat + is_dead_end
    }

    fn predict(&self) -> Direction {
        let mut ranked: Vec<(Direction, isize)> = vec![
            Direction::North,
            Direction::East,
            Direction::West,
            Direction::South,
        ].into_iter()
            .map(|d| (d, self.rank(d)))
            .collect();

        ranked.sort_by_key(|&(_, r)| r);

        ranked.get(0).map(|&(d, _)| d).unwrap()
    }

    pub fn draw(&self, printer: &Printer) {
        printer.with_color(ColorStyle::Highlight, |printer| for loc in &self.snake {
            printer.print(*loc, " ");
        });

        printer.with_color(ColorStyle::HighlightInactive, |printer| for loc in
            &self.food
        {
            printer.print(*loc, " ");
        });

        printer.print((1, 1), format!("{}", self.snake.len()).as_str());
    }

    pub fn step(&mut self, _direction: Option<Direction>) -> EventResult {
        use rand::*;

        let direction = self.predict();

        let next = self.next_loc(direction);

        if self.snake.iter().any(|&l| l == next) {
            // Lost!
            EventResult::with_cb(|siv| {
                siv.add_layer(Dialog::around(TextView::new("You lost!")).button(
                    "Ok",
                    |s| s.quit(),
                ));
            })
        } else {
            self.snake.push_front(next);

            self.prev_loc.push_front(next);

            if self.food.contains(&next) {
                self.food.remove(&next);
                for _ in 0..thread_rng().gen_range(1, 4) {
                    self.add_random_food();
                }
            } else {
                self.snake.pop_back();
            }

            self.direction = direction;

            let new_speed = self.snake.len() as u32;
            EventResult::with_cb(move |siv| siv.set_fps(new_speed))
        }
    }

    fn next_loc(&self, direction: Direction) -> (usize, usize) {
        let &(x, y) = self.snake.front().unwrap();
        let (x, y) = (x as isize, y as isize);

        let (new_x, new_y) = match direction {
            Direction::North => (x, y - 1),
            Direction::South => (x, y + 1),
            Direction::East => (x + 1, y),
            Direction::West => (x - 1, y),
        };

        let new_x = if new_x < 0 {
            self.size.0 - 1
        } else if new_x >= (self.size.0 as isize) {
            0
        } else {
            new_x as usize
        };

        let new_y = if new_y < 0 {
            self.size.1 - 1
        } else if new_y >= (self.size.1 as isize) {
            0
        } else {
            new_y as usize
        };

        (new_x, new_y)
    }

    fn add_random_food(&mut self) {
        use rand::*;
        self.food.insert((
            thread_rng().gen_range(0, self.size.0),
            thread_rng().gen_range(0, self.size.1),
        ));
    }
}
