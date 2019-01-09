#![feature(duration_float)]

use std::io::stdin;
use termion::screen::AlternateScreen;
use std::io::stdout;
use termion::raw::IntoRawMode;
use termion::raw::RawTerminal;
use std::io::Stdout;
use std::io::Write;
use std::thread::sleep;
use std::time::Duration;
use std::time::SystemTime;
use std::f32::consts::PI;
use termion::input::TermRead;
use termion::event::Key;


const MAP_WIDTH: usize = 16;
const MAP_HEIGHT: usize = 16;
const FOV: f32 = PI / 3.50f32;
const MAX_RENDERING_DEPTH: f32 = 16.0;

fn main() {


    let mut map =  Map::standard();
    map.set_line(4, "#.#.....#...##.#");


    let mut screen = init_screen();
    let (width, height) = termion::terminal_size().unwrap();
    let mut sb = ScreenBuffer::with_size(width as usize, height as usize);

    let mut player = PlayerCamera::default();
    player.x = 3.0;
    player.y = 5.0;


    let player_speed = 1.0;

    let mut frame_time: f32 = 0.0;

    'a: loop {
        let t1 = SystemTime::now();
        sb.render(&player, &map);
        sb.write_to_screen(&mut screen);

        sleep(Duration::new(0, 7_000_000));
        for c in stdin().keys().take(1) {
            match c.unwrap() {
                Key::Char('q') => break 'a,
                Key::Char(',') => {
                    player.x += f32::sin(player.angle) * player_speed * frame_time;
                    player.y += f32::cos(player.angle) * player_speed * frame_time;
                },
                Key::Char('a') => { // TODO something about left + right strafing doesn't feel right
                    player.x += f32::sin(player.angle) * player_speed * frame_time;
                    player.y -= f32::cos(player.angle) * player_speed * frame_time;
                },
                Key::Char('o') => {
                    player.x -= f32::sin(player.angle) * player_speed * frame_time;
                    player.y -= f32::cos(player.angle) * player_speed * frame_time;
                },
                Key::Char('e') => {
                    player.x -= f32::sin(player.angle) * player_speed * frame_time;
                    player.y += f32::cos(player.angle) * player_speed * frame_time;
                },
                Key::Left => player.angle -= 0.1,
                Key::Right => player.angle += 0.1,
                _ => {}
            }
        }
        frame_time = t1.elapsed().unwrap().as_float_secs() as f32;
    }


    // reset
    write!(screen, "{}", termion::cursor::Show).unwrap();
    std::mem::drop(screen);
//    println!("{:?}", player);
}

fn init_screen() -> AlternateScreen<RawTerminal<Stdout>> {
    let mut screen: AlternateScreen<RawTerminal<Stdout>> = AlternateScreen::from(stdout().into_raw_mode().unwrap());
    write!(screen, "{}", termion::cursor::Hide).unwrap();
    screen
}




#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
enum Space {
    Wall,
    Empty
}

impl From<char> for Space {
    fn from(c: char) -> Self {
        match c {
            '.' => Space::Empty,
            '#' => Space::Wall,
            _ => panic!("undefined space type")
        }
    }
}

impl Into<char> for Space {
    fn into(self) -> char {
        match self {
            Space::Wall  => '#',
            Space::Empty => '.'
        }
    }
}

impl Default for Space {
    fn default() -> Self {
        Space::Empty
    }
}

#[derive(Default, Debug)]
struct Map([[Space; MAP_WIDTH]; MAP_HEIGHT]);

impl Map {
    fn standard() -> Self {
        let mut map = Map::default();
        map.set_line(0,  "################");
        map.set_line(15, "################");
        (1..=14).for_each(|line| map.set_line(line, "#..............#"));
        map
    }
    fn set_line(&mut self, line: usize, s: &str) {
        assert_eq!(s.len(), MAP_WIDTH);
        assert!(line < MAP_HEIGHT);
        s.chars()
            .enumerate()
            .for_each(|(x, c)|{
                self.0[line][x] = c.into()
            });
    }

    fn get_distances(&self, player: &PlayerCamera, screen_width: usize) -> Vec<f32> {
        (0..screen_width) // for each vertical row
            .map(|row_index| {
                let ray_angle = (player.angle - FOV / 2.0) + (row_index as f32 / screen_width as f32) * FOV;

                let mut dist_to_wall = 0.0;
                const STEP_SIZE: f32 = 0.1;

                let mut hit_wall = false;		// Set when ray hits wall block

                let eye_x = f32::sin(ray_angle); // Unit vector for ray in player space
                let eye_y = f32::cos(ray_angle);

                while !hit_wall && dist_to_wall < MAX_RENDERING_DEPTH {
                    dist_to_wall += STEP_SIZE;
                    let test_x = (player.x + eye_x * dist_to_wall) as isize;
                    let test_y = (player.y + eye_y * dist_to_wall) as isize;

                    if test_x < 0 ||  test_x >= MAP_WIDTH as isize || test_y < 0 || test_y >= MAP_HEIGHT as isize {
                        hit_wall = true;
                        dist_to_wall = MAX_RENDERING_DEPTH;
                    } else {
                        if self.0[test_x as usize][test_y as usize] == Space::Wall {
                            hit_wall = true
                        }
                    }
                }

                dist_to_wall
            })
            .collect()
    }
}

#[derive(Debug)]
struct ScreenBuffer(Vec<Vec<char>>);

impl ScreenBuffer {

    fn with_size(width: usize, height: usize) -> Self {
        ScreenBuffer(vec![vec!['x'; width]; height])
    }

    fn write_to_screen(&self, screen: &mut AlternateScreen<RawTerminal<Stdout>>) {
        use termion::clear;
        use termion::cursor;
        write!(screen, "{}", clear::All).unwrap();
        self.0.iter().enumerate().for_each(|(i, vc)| {
            write!(screen, "{}{}", cursor::Goto(1, i as u16 + 1), vc.iter().cloned().collect::<String>()).unwrap();
        });
        screen.flush().unwrap();
    }

    fn render(&mut self, player: &PlayerCamera, map: &Map) {
        let screen_width = self.0[0].len();
        let screen_height = self.0.len();

       map.get_distances(player, screen_width)
            .into_iter()
            .enumerate() // 0..screen width effectively
            .for_each(|(x_index, distance)| {
                let (ceiling, floor) = get_ceiling_and_floor_heights_from_distance(distance, screen_height);

                (0..screen_height).for_each(|y_index| {
                    if y_index <= ceiling {
                        self.0[y_index][x_index] = ' ';
                    } else if y_index > ceiling && y_index <= floor {
                        self.0[y_index][x_index] = render_wall(distance)
                    } else {
                        self.0[y_index][x_index] = render_floor(y_index, screen_height)
                    }
                });
        });

    }
}

/// ceiling, floor
fn get_ceiling_and_floor_heights_from_distance(distance: f32, screen_height: usize) -> (usize, usize) {
    let ceiling = ((screen_height as f32 / 2.0) - screen_height as f32 / distance) as usize;
    let floor = screen_height.saturating_sub(ceiling);
    (ceiling, floor)
}

fn render_wall(distance: f32) -> char {
    match distance {
        x if x < MAX_RENDERING_DEPTH / 4.0 => '█',
        x if x < MAX_RENDERING_DEPTH / 3.0 => '▓',
        x if x < MAX_RENDERING_DEPTH / 2.0 => '▒',
        x if x < MAX_RENDERING_DEPTH =>       '░',
        _ =>                                  ' '
    }
}

fn render_floor(y: usize, screen_height: usize) -> char {
    let b = 1.0 - ((y as f32 - screen_height as f32/2.0) / (screen_height as f32/ 2.0));
    match b {
        x if x < 0.25 => '#',
        x if x < 0.5 =>  'x',
        x if x < 0.65 => ',',
        x if x < 0.75 => '.',
        x if x < 0.8 =>  '_',
        x if x < 0.9 =>  '-',
        _ =>             ' '
    }
}


#[derive(Default, Debug)]
struct PlayerCamera {
    x: f32,
    y: f32,
    angle: f32
}
