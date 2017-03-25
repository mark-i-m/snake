#![feature(rand)]

extern crate cursive;

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
}

fn main() {
    let mut siv = Cursive::new();
    siv.set_fps(2);

    let canvas = BoxView::with_full_screen(Canvas::new(State::new(siv.screen_size().pair()))
        .with_draw(|printer, state| state.draw(printer))
        .with_on_event(|event, state| match event {
            Event::Char('q') => EventResult::Ignored,
            Event::Key(Key::Up) => state.step(Some(Direction::North)),
            Event::Key(Key::Down) => state.step(Some(Direction::South)),
            Event::Key(Key::Left) => state.step(Some(Direction::West)),
            Event::Key(Key::Right) => state.step(Some(Direction::East)),
            _ => state.step(None),
        }));

    siv.add_fullscreen_layer(canvas);

    siv.add_global_callback('q', |s| s.quit());

    siv.run();
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
        };

        state.add_random_food();

        state
    }

    pub fn draw(&self, printer: &Printer) {
        printer.with_color(ColorStyle::Highlight, |printer| for loc in &self.snake {
            printer.print(*loc, " ");
        });

        printer.with_color(ColorStyle::HighlightInactive,
                           |printer| for loc in &self.food {
                               printer.print(*loc, "O");
                           });
    }

    pub fn step(&mut self, direction: Option<Direction>) -> EventResult {
        let direction = direction.unwrap_or(self.direction);

        let next = self.next_loc(direction);

        if self.snake.iter().any(|l| *l == next) {
            // Lost!
            EventResult::with_cb(|siv| {
                siv.add_layer(Dialog::around(TextView::new("You lost!"))
                    .button("Ok", |s| s.quit()));
            })
        } else {
            self.snake.push_front(next);

            if self.food.contains(&next) {
                self.food.remove(&next);
                self.add_random_food();
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
            self.size.0
        } else if new_x >= (self.size.0 as isize) {
            0
        } else {
            new_x as usize
        };

        let new_y = if new_y < 0 {
            self.size.1
        } else if new_y >= (self.size.1 as isize) {
            0
        } else {
            new_y as usize
        };

        (new_x, new_y)
    }

    fn add_random_food(&mut self) {
        use std::__rand::*;
        self.food.insert((std::__rand::thread_rng().gen_range(0, self.size.0),
                          std::__rand::thread_rng().gen_range(0, self.size.1)));
    }
}
