use crate::screen_clear;
use nanos_sdk::screen::sdk_bagl_hal_draw_rect;
use nanos_sdk::screen::sdk_screen_update;

const BLACK: u32 = 0;
const WHITE: u32 = 1;

const SCREEN_WIDTH: i32 = 127;
const SCREEN_HEIGHT: i32 = 63;

// Maximum size the snake can reach. Will exit if snake is bigge than this value.
const SNAKE_MAX_SIZE: u16 = 200;

// Size of the snake when the game starts.
const SNAKE_STARTING_SIZE: u8 = 3;

/// Game representation.
pub struct Game {
    snake: Snake,
    target: Point,
}

impl Game {
    /// An iteration in the game. Designed to be call on every ticker event.
    pub fn step(&mut self) {
        // Clear the snake from the screen.
        self.snake.body[..self.snake.size]
            .iter()
            .for_each(|point| sdk_bagl_hal_draw_rect(BLACK, point.x, point.y, 1, 1));

        // Advance the snake.
        self.snake.step(self.target);

        // Check if the target was eaten by the snake.
        if self.snake.eaten {
            // Target was eaten, generate a new target.
            let mut target = Point::random();
            // Check that generated target is not IN the snake's body. If it is, generate a new one...
            while self.snake.body.iter().any(|&point| point == target) {
                target = Point::random();
            }

            // Update the target
            self.target = target;
            // Draw the target on screen.
            sdk_bagl_hal_draw_rect(WHITE, target.x, target.y, 1, 1);
        }

        // Draw the snake in white.
        self.snake.body[..self.snake.size]
            .iter()
            .for_each(|point| sdk_bagl_hal_draw_rect(WHITE, point.x, point.y, 1, 1));

        sdk_screen_update();
    }

    /// Create a new game.
    pub fn new() -> Self {
        screen_clear();

        let target = Point::random();
        sdk_bagl_hal_draw_rect(WHITE, target.x, target.y, 1, 1);
        sdk_screen_update();

        Self {
            snake: Snake::new(),
            target,
        }
    }

    pub fn turn_right(&mut self) {
        self.snake.direction = match self.snake.direction {
            Direction::Left => Direction::Down,
            Direction::Up => Direction::Left,
            Direction::Right => Direction::Up,
            Direction::Down => Direction::Right,
        }
    }

    pub fn turn_left(&mut self) {
        self.snake.direction = match self.snake.direction {
            Direction::Left => Direction::Up,
            Direction::Up => Direction::Right,
            Direction::Right => Direction::Down,
            Direction::Down => Direction::Left,
        }
    }
}

#[derive(Default, Clone, Copy, PartialEq)]
struct Point {
    x: i32,
    y: i32,
}

impl Point {
    /// Creates a {0, 0} point.
    const fn zero() -> Self {
        Self { x: 0, y: 0 }
    }

    fn new(x: i32, y: i32) -> Self {
	    Self {
		    x, y
	    }
    }

    /// Generates a random point. This point will be WITHIN the screen.
    fn random() -> Self {
        let mut x = nanos_sdk::random::rand_u8() as i32;
        let width = 0..SCREEN_WIDTH;
        // make sure X is in the screen.
        while !width.contains(&x) {
            x = nanos_sdk::random::rand_u8() as i32;
        }

        let height = 0..SCREEN_HEIGHT;

        let mut y = nanos_sdk::random::rand_u8() as i32;

        // make sure Y is in the screen.
        while !height.contains(&y) {
            y = nanos_sdk::random::rand_u8() as i32;
        }

        Self { x, y }
    }
}

/// Direction of the snake.
enum Direction {
    Left,
    Up,
    Right,
    Down,
}

pub struct Snake {
    body: [Point; SNAKE_MAX_SIZE as usize],
    size: usize,
    direction: Direction,
    eaten: bool,
}

impl Snake {
    /// Creates a new snake. The snake starts at a random position, and will go towards the right.
    fn new() -> Self {
	// Create the body. Fill it with SNAKE_STARTING_SIZE points.
        let mut body = [Point::zero(); SNAKE_MAX_SIZE as usize];

	// Start by generating a random the last point.
	body[SNAKE_STARTING_SIZE as usize - 1] = Point::random();
	
	// Start the range at SNAKE_STARTING_SIZE - 1 because we already initiated the last point.
	let range = 0..(SNAKE_STARTING_SIZE - 1) as usize;

	// Fill in the rest of the body, starting from the last point to the first one.
	// Only modify x value, y will stay the same so that the snake starts on a horizontal line.
	for i in range.rev() {
		body[i] = Point::new(body[i + 1].x + 1, body[i + 1].y);
	}

        Self {
            body,
            size: SNAKE_STARTING_SIZE as usize,
            direction: Direction::Right,
            eaten: false,
        }
    }

    /// Perform a single step for the snake.
    /// Check `snake.eaten` after `step` to see if the snake ate the target during this step.
    fn step(&mut self, target: Point) {
        // Reset the eaten flag to false.
        self.eaten = false;

        let mut next_head = self.body[0];
        // Move the next_head.
        match self.direction {
            Direction::Right => {
                if next_head.x == SCREEN_WIDTH {
                    next_head.x = 0;
                } else {
                    next_head.x += 1;
                }
            }
            Direction::Left => {
                if next_head.x == 0 {
                    next_head.x = SCREEN_WIDTH;
                } else {
                    next_head.x -= 1;
                }
            } // check underflow
            Direction::Down => {
                if next_head.y == 0 {
                    next_head.y = SCREEN_HEIGHT;
                } else {
                    next_head.y -= 1;
                }
            } // check underflow
            Direction::Up => {
                if next_head.y == SCREEN_HEIGHT {
                    next_head.y = 0;
                } else {
                    next_head.y += 1;
                }
            } // check overflow
        }

        // Check if we are about to eat the target.
        if next_head == target {
            // We ate the target: set the self.eaten flag accordingly.
            self.eaten = true;
            // We juste ate the target: grow in size.
            self.size += 1;
        }

        // Move the body (limited by `self.size`) on cell to the right.
        // The last cell will get put in the first cell (wrap around).
        self.body[..self.size].rotate_right(1);

        // Check if `next_head` is going to collide with the body.
        if self.body[..self.size]
            .iter()
            .any(|&point| point == next_head)
        {
            nanos_sdk::exit_app(1);
        }

        // Set the first cell (the head) to be the `next_head` we computed before.
        self.body[0] = next_head;
    }
}
