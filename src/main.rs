use crossterm::event::KeyCode;
use crossterm::style::Stylize;
use crossterm::{
    cursor,
    event::{read, Event},
    //screen::RawScreen,
    style::{style, Color, ContentStyle, StyledContent},
    terminal::{Clear, ClearType},
    QueueableCommand,
};
use rand::Rng;
use std::collections::BTreeSet;
use std::collections::VecDeque;
use std::io::{stdout, Stdout, Write};
use std::sync::{Arc, Mutex};
use std::time::Duration;
use std::{process, thread};


enum MoveAct {
    Move,
    Grow,
}

#[derive(Debug, Clone, PartialEq)]
struct SnakeChain {
    x: isize,
    y: isize,
}
#[derive(Debug)]
enum Direction {
    Left,
    Right,
    Up,
    Down,
}

#[derive(Debug, Eq, PartialEq, PartialOrd, Ord)]
struct Apple {
    x: isize,
    y: isize,
}

struct Apples {
    apples: BTreeSet<Apple>,
    apple_symbol: StyledContent<char>,
    percent: f32,
}
#[warn(non_snake_case)]
struct Snake {
    moveDirection: Arc<Mutex<Direction>>,
    body: VecDeque<SnakeChain>,
    chainSymbol: StyledContent<char>,
}
#[warn(non_snake_case)]
struct Game {
    width: isize,
    height: isize,
    speed_msec: u64,
    term: Stdout,
    snake: Snake,
    apples: Apples,
    playgroundSymbol: char,
    playgroundColor: ContentStyle,
}

impl Direction {
    fn turn_left(&mut self) {
        use Direction::*;
        *self = match *self {
            Right => Up,
            Left => Down,
            Up => Left,
            Down => Right,
        }
    }
    
    fn turn_right(&mut self) {
        use Direction::*;
        *self = match *self {
            Right => Down,
            Left => Up,
            Up => Right,
            Down => Left,
        }
    }
}

impl Apples {
    fn new(percent: f32, width: isize, height: isize, snake: &Snake) -> Apples {
        let mut apples = BTreeSet::new();
        let apples_cnt = (width + height - snake.body.len() as isize) as f32 * percent;

        let mut r = rand::thread_rng();
        for _ in 0..apples_cnt as usize {
            loop {
                let x = r.gen_range(0..width);
                let y = r.gen_range(0..height);
                let c = Apple {x, y};
                let s = SnakeChain {x, y};
                if !snake.body.contains(&s) && !apples.contains(&c) {
                        apples.insert(c);
                        break;
                }
            }
        }

        Apples {
            apple_symbol: style('x').with(Color::Red),
            apples,
            percent,
        }
    }

    fn draw(&self, term: &mut Stdout) {
        for apple in &self.apples {
            term.queue(cursor::MoveTo(apple.x as u16, apple.y as u16))
                .unwrap();
            print!("{}", self.apple_symbol);
            term.flush().unwrap();
        }
    }

    fn remove(&mut self, apple: Apple) {
        self.apples.remove(&apple);
    }

    fn add(&mut self, term: &mut Stdout, width: isize, height: isize, snake: &Snake) {
        let mut apples_cnt = ((width + height - snake.body.len() as isize) as f32 * self.percent) as usize;
        if apples_cnt < 1 {
            apples_cnt = 1;
        }
        if apples_cnt <= self.apples.len() { return; }

        let mut r = rand::thread_rng();
        loop {
            let x = r.gen_range(0..width);
            let y = r.gen_range(0..height);
            let c = Apple {x, y};
            let s = SnakeChain {x, y};
            if !snake.body.contains(&s) && !self.apples.contains(&c) {
                self.apples.insert(c);
                term.queue(cursor::MoveTo(x as u16, y as u16)).unwrap();
                print!("{}", self.apple_symbol);
                term.flush().unwrap();
                break;
            }
        }
    }
}

impl Snake {
    fn new(left: isize, top: isize, len: usize, move_direction: Direction) -> Snake {
        let mut body = VecDeque::new();
        let x_inc;
        let y_inc;

        match move_direction {
            Direction::Left => {
                x_inc = -1;
                y_inc = 0
            }
            Direction::Right => {
                x_inc = 1;
                y_inc = 0
            }
            Direction::Up => {
                x_inc = 0;
                y_inc = -1
            }
            Direction::Down => {
                x_inc = 0;
                y_inc = 1
            }
        }
        for i in 0..len as isize {
            body.push_front(SnakeChain {
                x: left + i * x_inc,
                y: top + i * y_inc,
            });
        }

        let moveDirection = Arc::new(Mutex::new(move_direction));
        Snake {
            moveDirection,
            body,
            chainSymbol: style('o').blue(),
        }
    }

    fn draw (&self, term: &mut Stdout) {
        for snakechain in &self.body {
            term.queue(cursor::MoveTo(snakechain.x as u16, snakechain.y as u16)).unwrap();
            print!("{}", self.chainSymbol);
        } 
    }

    fn cut_tail(&mut self, term: &mut Stdout, hide: &StyledContent<char>) {
        let tail = self.body.pop_back().unwrap();
        term.queue(cursor::MoveTo(tail.x as u16, tail.y as u16)).unwrap();
        print!("{}", hide);
        term.flush().unwrap();
    }

    fn add_head(&mut self, term: &mut Stdout) {
        let mut head = self.body[0].clone();
        match *self.moveDirection.lock().unwrap() {
            Direction::Left => {
                head.x -= 1;
            }

            Direction::Right => {
                head.x += 1;
            }

            Direction::Up => {
                head.y -= 1;
            }

            Direction::Down => {
                head.y += 1;
            }
        }
        if head.x >= 0 && head.y >= 0 {
            term.queue(cursor::MoveTo(head.x as u16, head.y as u16)).unwrap();
            print!("{}", self.chainSymbol);
            term.flush().unwrap();
        }
        self.body.push_front(head);
    }

    fn _move(&mut self, mut term: &mut Stdout, hide: &StyledContent<char>, act: MoveAct) {
        if let MoveAct::Move = act {
            self.cut_tail(&mut term, hide);
        }
        self.add_head(&mut term);
    }
}

impl Game {
    fn new(width: isize, height: isize, speed_msec: u64) -> Game {
        let snake = Snake::new(0, height/2, 7, Direction::Right);

        let apples = Apples::new(
            0.3,
            width,
            height,
            &snake,
        );

        let term = stdout();

        Game {
            width,
            height,
            speed_msec,
            term,
            snake,
            apples,
            playgroundSymbol: '.',
            playgroundColor: ContentStyle::new(),
        }
    }

    fn draw_playground(&mut self) {
        self.term.queue(Clear(ClearType::All)).unwrap();
        self.term.queue(cursor::MoveTo(0,0)).unwrap();
        self.term.queue(cursor::Hide).unwrap();
        self.term.flush();
        let r = self.playgroundSymbol.to_string().repeat(self.width as usize);
        for _ in 0..self.height {
            println!("{}", self.playgroundColor.apply(&r));
            self.term.flush().unwrap();
        }
    }

    fn start_key_press_handler(&self) {
        let mut term = stdout();
        let dr = self.snake.moveDirection.clone();
        thread::spawn(move || {
            loop {
                if let Event::Key(event) = read().unwrap() {
                    match event.code {
                        KeyCode::Left => dr.lock().unwrap().turn_left(),
                        KeyCode::Right => dr.lock().unwrap().turn_right(),
                        KeyCode::Esc => {
                            term.queue(cursor::Show).unwrap();
                            term.flush().unwrap();
                            process::exit(0);
                        }
                        _ => (),
                    }
                }
            }
        });
    }

    fn snekMeetWall(&mut self) -> bool {
        let head = self.snake.body.pop_front().unwrap();
        let mut res = false;
        if head.x > self.width - 1 || head.x < 0 {
            res = true;
        }
        if head.y > self.height - 1 || head.y < 0 {
            res = true;
        }
        self.snake.body.push_front(head);
        res
    }

    fn snekMeetApple(&mut self) -> Option<Apple> {
        let head = self.snake.body[0].clone();
        let apple = Apple {
            x: head.x,
            y: head.y,
        };
        if self.apples.apples.contains(&apple) {
            return Some(apple)
        }
        None
    }

    fn play (&mut self) {
        self.draw_playground();
        self.snake.draw(&mut self.term);
        self.apples.draw(&mut self.term);
        self.start_key_press_handler();
        let hide = self.playgroundColor.apply(self.playgroundSymbol);
        let mut act = MoveAct::Move;
        
        loop {
            self.snake._move(&mut self.term, &hide, act);
            act = MoveAct::Move;
            
            if self.snekMeetWall() {
                break;
            }

            if let Some(apple) = self.snekMeetApple() {
                act = MoveAct::Grow;
                self.apples.remove(apple);
                self.apples.add(&mut self.term, self.width, self.height, &self.snake);
            }
            thread::sleep(Duration::from_millis(self.speed_msec));
        }
        
        self.term.queue(cursor::Show).unwrap();
        self.term.queue(Clear(ClearType::All)).unwrap();
        self.term.flush().unwrap();
        
    }


}


fn main() {
    let mut game = Game::new(
        100,
        50,
        100
    );
    game.play();
}
