
extern crate rand;
extern crate cursive;
extern crate rayon;

use std::cmp::min;
use std::collections::{LinkedList, HashSet};
use std::time::{Duration, Instant};

use cursive::{Cursive, Printer};
use cursive::views::{BoxView, Canvas, TextView, Dialog};
use cursive::event::{/*Key,*/ Event, EventResult};
use cursive::theme::ColorStyle;

use rayon::prelude::*;

const MAX_DEPTH: usize = 13;
const LOSS_PENALTY: isize = 100_000_000;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Direction {
    North,
    South,
    East,
    West,
}

type Snake = LinkedList<(usize, usize)>;

#[derive(Debug)]
struct State {
    size: (usize, usize),
    direction: Direction,
    snake: Snake,
    food: HashSet<(usize, usize)>,
}

#[derive(Debug, Clone)]
struct SearchState<'s> {
    original: &'s State,
    search_snake: Snake,
    search_food: HashSet<(usize, usize)>,
    ate: bool,
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
        };

        state.add_random_food();

        state
    }

    fn choose_next_action(&self) -> Direction {
        let fps = self.snake.len() as u64;
        let time_bound = Instant::now() + Duration::from_millis(1000u64 / fps);
        let search_state = SearchState::new(self);
        let mut ranked: Vec<(Direction, isize)> = vec![
            Direction::North,
            Direction::East,
            Direction::West,
            Direction::South,
        ].into_iter()
            .map(|d| (d, search_state.step(d)))
            .map(|(d, s)| (d, s.rank(d, time_bound, MAX_DEPTH)))
            .collect();

        ranked.sort_by_key(|&(_, r)| r);

        // There should always be one direction that is an automatic loss
        if !ranked.iter().any(|&(_, x)| x >= LOSS_PENALTY) && self.snake.len() > 2 {
            println!("{:?}", ranked);
            loop {}
        }

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

        let direction = self.choose_next_action();

        // Move the snake
        let next = State::next_loc(&self.snake, direction, self.size);
        self.snake.push_front(next);

        if self.food.contains(&next) {
            self.food.remove(&next);
            for _ in 0..thread_rng().gen_range(1, 4) {
                self.add_random_food();
            }
        } else {
            self.snake.pop_back();
        }

        self.direction = direction;

        // Check for loss
        let head = *self.snake.front().unwrap();
        if self.snake.iter().skip(1).any(|&l| l == head) {
            // Lost!
            EventResult::with_cb(|siv| {
                siv.add_layer(Dialog::around(TextView::new("You lost!")).button(
                    "Ok",
                    |s| s.quit(),
                ));
            })
        } else {
            let new_speed = self.snake.len() as u32;
            EventResult::with_cb(move |siv| siv.set_fps(new_speed))
        }
    }

    fn add_random_food(&mut self) {
        use rand::*;
        self.food.insert((
            thread_rng().gen_range(0, self.size.0),
            thread_rng().gen_range(0, self.size.1),
        ));
    }

    fn next_loc(snake: &Snake, direction: Direction, size: (usize, usize)) -> (usize, usize) {
        let &(x, y) = snake.front().unwrap();
        let (x, y) = (x as isize, y as isize);

        let (new_x, new_y) = match direction {
            Direction::North => (x, y - 1),
            Direction::South => (x, y + 1),
            Direction::East => (x + 1, y),
            Direction::West => (x - 1, y),
        };

        let new_x = if new_x < 0 {
            size.0 - 1
        } else if new_x >= (size.0 as isize) {
            0
        } else {
            new_x as usize
        };

        let new_y = if new_y < 0 {
            size.1 - 1
        } else if new_y >= (size.1 as isize) {
            0
        } else {
            new_y as usize
        };

        (new_x, new_y)
    }
}

impl<'s> SearchState<'s> {
    pub fn new(state: &'s State) -> SearchState<'s> {
        SearchState {
            original: state,
            search_snake: state.snake.clone(),
            search_food: state.food.clone(),
            ate: false,
        }
    }

    // Higher rank is worse
    pub fn rank(&self, direction: Direction, time_bound: Instant, depth_bound: usize) -> isize {
        let mut min_rank = LOSS_PENALTY; // We want to do better than losing

        let mut queue = LinkedList::new();
        queue.push_back((self.clone(), 0, 0));

        while !queue.is_empty() {
            let (next, parent_score, depth) = queue.pop_front().unwrap();

            // The rank for `next`
            let mut rank = parent_score;

            // Have we lost?
            if next.is_loss() {
                // Obviously, we don't want to go down this path :P
                continue;
            }

            // Have we eaten?
            rank += if next.is_eat() { -1000 } else { 0 };

            // Are we out of time?
            if Instant::now() >= time_bound {
                rank += self.quick_rank(direction);
                min_rank = min(min_rank, rank);
                continue;
            }

            // Too far in the future?
            if depth >= depth_bound {
                rank += self.quick_rank(direction);
                min_rank = min(min_rank, rank);
                continue;
            }

            // Enqueue all possible next actions to explore
            let successors = vec![
                Direction::North,
                Direction::East,
                Direction::West,
                Direction::South,
            ].into_iter()
                .map(|d| (self.step(d), rank, depth + 1));

            queue.extend(successors);
        }

        return min_rank;
    }

    fn is_loss(&self) -> bool {
        let head = *self.search_snake.front().unwrap();
        self.search_snake.iter().skip(1).any(|&b| b == head)
    }

    fn is_eat(&self) -> bool {
        self.ate
    }

    fn quick_rank(&self, direction: Direction) -> isize {
        let head = *self.search_snake.front().unwrap();
        let food_rank = self.search_food
            .iter()
            .map(|&food| distance(self.original.size, head, food) as isize)
            .min()
            .unwrap_or(0);

        let body_size = self.search_snake.len();
        let body_rank: isize = self.search_snake
            .iter()
            .take(body_size - 1)
            .map(|&body| distance(self.original.size, head, body) as isize)
            .sum();

        // Incentives doing the same thing again
        let same_direction = if direction == self.original.direction {
            -1
        } else {
            0
        };

        // Avoid coming close to yourself
        let is_dead_end = if self.search_snake
            .iter()
            .take(body_size - 1)
            .map(|&body| distance(self.original.size, head, body) as isize)
            .filter(|&d| d < 5)
            .count() > 1
        {
            10000
        } else {
            0
        };

        food_rank + same_direction - (body_rank / body_size as isize) + is_dead_end
    }

    pub fn step(&self, direction: Direction) -> SearchState {
        let mut new_snake = self.search_snake.clone();
        let mut new_food = self.search_food.clone();

        let next = State::next_loc(&new_snake, direction, self.original.size);

        new_snake.push_front(next);

        let ate = if new_food.contains(&next) {
            new_food.remove(&next);
            true
        } else {
            new_snake.pop_back();
            false
        };

        SearchState {
            original: self.original,
            search_snake: new_snake,
            search_food: new_food,
            ate,
        }
    }
}
