use crossterm::event::{self, KeyCode};
use ratatui::{
    style::{Color, Stylize},
    prelude::{CrosstermBackend, Terminal, Marker,},
    widgets::canvas::*,
    Frame
};
use std::{io::{Result, Stdout}, u128, path::PathBuf};
use std::time::Instant;
use std::{path::Path, fs::File};
use std::io::{self, BufRead, BufReader};
use rodio::{Decoder, OutputStream, Sink};

enum Score {
    None,
    Miss,
    Great,
    Perfect,
    Ok,
}

struct HitObject {
    time: u128,
    x: i32,
    y: i32,
    combo: i32,
    score: Score,
}

pub struct BeatMap {
    preempt: u128,
    cs: f64,
    od: f64,
    track: PathBuf,
    objs: Vec<HitObject>,
    hit_box: (f64, f64),
}

impl HitObject {
    fn score(&self, od: f64, hit: u128) -> Score{
        match self.score {
            Score::None => {},
            Score::Miss => { return Score::Miss; }
            Score::Ok => { return Score::Ok; }
            Score::Great => { return Score::Great; }
            Score::Perfect => { return Score::Perfect; }
        }

        let abs = if hit < self.time { self.time-hit } else { hit-self.time };
        if abs <= 80 - (6.0 * od) as u128 {
            return  Score::Perfect;
        }
        
        if abs <= 140 - (8.0 * od) as u128 {
            return Score::Great;
        }

        if abs <= 200 - (10.0 * od) as u128 {
            return Score::Ok;
        }
        
        return Score::None;
    }
}

impl BeatMap {
    fn set_ar(&mut self, ar: f64) {
        if ar < 5.0 {
            self.preempt = 1200 + (600.0 * ((5.0 - ar)/5.0)) as u128;
        }
        else {
            self.preempt = 1200 - (750.0 * ((ar - 5.0)/5.0)) as u128;
        }
    }

    fn set_cs(&mut self, cs: f64) {
        self.cs = 54.4 - 4.48 * cs;
    }

    fn draw_obj(&self, ctx: &mut Context, time: u128, obj: &HitObject) {
        if time + self.preempt < obj.time || obj.time + 200 - (10.0 * self.od) as u128 + self.preempt < time {
            return;
        }
        let t_x = obj.x as f64;
        let t_y = 384.0 - obj.y as f64;

        match obj.score {
            Score::Perfect => {
                return;
            }
            Score::Great => {
                ctx.print(t_x, t_y, "100".bold().blue()); 
                return;
            }
            Score::Ok => {
                ctx.print(t_x, t_y, "50".bold().light_red()); 
                return;
            }
            Score::Miss => {
                ctx.print(t_x, t_y, "X".bold().red()); 
                return;
            }
            Score::None => {}
        }

        if time <= obj.time {
            ctx.draw(&Circle {
                x: t_x,
                y: t_y,
                radius: self.cs +  5.0 * self.cs * ((obj.time - time) as f64 / self.preempt as f64),
                color: Color::White,
            });
        }
        ctx.draw(&Circle {
            x: t_x,
            y: t_y,
            radius: self.cs,
            color: Color::Yellow,
        });
        ctx.print(t_x, t_y, obj.combo.to_string().yellow()); 
    }

    fn draw_game(&mut self, time: u128, frame: &mut Frame) {
        let area = frame.size();
        self.hit_box = (512.0/area.width as f64, 384.0/area.height as f64);
        frame.render_widget(
            Canvas::default()
                .background_color(Color::Black)
                .marker(Marker::Braille)
                .paint(move |ctx| {
                    for obj in &self.objs {
                        self.draw_obj(ctx, time, &obj);
                    }
                }) 
                .x_bounds([0.0, 512.0])
                .y_bounds([0.0, 384.0]),
            area,);
    }

    fn score_miss(&mut self, time: u128) {
        for obj in &mut self.objs {
            if obj.time + 200 - (10.0 * self.od) as u128 >= time {
                break;
            }
            if let Score::None = obj.score {
                obj.score = Score::Miss;
            }
        }
    }

    fn score_hit(&mut self, time: u128, pointer: (u16, u16)) {
        for obj in &mut self.objs {
            let x = self.hit_box.0 * pointer.0 as f64;
            let y = self.hit_box.1 * pointer.1 as f64;

            if dist_sq((x, y), (obj.x as f64, obj.y as f64)) > self.cs*self.cs {
                continue;
            }

            if let Score::None = obj.score {
                obj.score = obj.score(self.od, time);
                break;
            }
        }
    }

    pub fn play_map(path: &Path, terminal: &mut Terminal<CrosstermBackend<Stdout>>) -> Result<()> {  
        let mut map = load_map(path);

        let (_stream, stream_handle) = OutputStream::try_default().unwrap();
        let file = BufReader::new(File::open(map.track.as_path()).unwrap());
        let source = Decoder::new(file).unwrap();
        let sink = Sink::try_new(&stream_handle).unwrap();
        sink.append(source);
        let start = Instant::now(); // TODO better way of sync

        let mut pointer = (0, 0);
        while !sink.empty() {
            let frame_time = start.elapsed().as_millis();

            terminal.draw(|frame| {
                map.draw_game(frame_time, frame);
            })?;
            if event::poll(std::time::Duration::from_millis(16))? {
                match event::read()? {
                    event::Event::Key(key) => {
                        if key.kind != event::KeyEventKind::Press {
                            continue;
                        }

                        if key.code == KeyCode::Char('a') || key.code == KeyCode::Char('d') {
                            map.score_hit(frame_time, pointer);
                        }
                        if key.code == KeyCode::Char('q') || key.code == KeyCode::Char('Q') {
                            break;
                        }

                    },
                    event::Event::Mouse(mouse) => {                        
                        if mouse.kind == event::MouseEventKind::Moved {
                            pointer = (mouse.column, mouse.row);
                        }
                    },
                    _ => (),
                }
            }
            map.score_miss(frame_time);
        }
        Ok(())
    }
}


fn dist_sq(a: (f64, f64), b: (f64, f64)) -> f64 {
    (a.0 - b.0)*(a.0 - b.0) + (a.1 - b.1)*(a.1 - b.1)
}

fn load_map(path: &Path) -> BeatMap{
    let lines = io::BufReader::new(File::open(path).unwrap()).lines();
    let mut parse_objs = false;

    let mut t_combo: i32 = 1;
    let mut res_objs: Vec<HitObject> = vec![];
    let mut res_track = PathBuf::new();
    let mut res_cs = 4.5;
    let mut res_od = 7.0;
    let mut res_ar = 9.0;

    for line in lines {
        let line = line.unwrap();

        if line.starts_with("AudioFilename") {
            let temp: Vec<&str> = line.split(" ").collect();
            res_track = path.parent().unwrap().join(temp[1]);
        }
        if line.starts_with("CircleSize") {
            let temp: Vec<&str> = line.split(":").collect();
            res_cs = temp[1].parse::<f64>().unwrap();
        }
        if line.starts_with("OverallDifficulty") {
            let temp: Vec<&str> = line.split(":").collect();
            res_od = temp[1].parse::<f64>().unwrap();
        }
        if line.starts_with("ApproachRate") {
            let temp: Vec<&str> = line.split(":").collect();
            res_ar = temp[1].parse::<f64>().unwrap();
        }
        if line.starts_with("[") {
            parse_objs = line.starts_with("[HitObjects]");
            continue;
        }

        if !parse_objs {
            continue;
        }

        let temp: Vec<&str> = line.split(",").collect();
        /*if temp[3].parse::<i32>().unwrap()&1 == 0 {  
            continue;
        }*/

        if temp[3].parse::<i8>().unwrap()&4 != 0 {  
            t_combo = 1;
        }

        res_objs.push(HitObject{
            x: temp[0].parse::<i32>().unwrap(),
            y: temp[1].parse::<i32>().unwrap(),
            time: temp[2].parse::<u128>().unwrap(),
            combo: t_combo,
            score: Score::None,
        });
        t_combo += 1;
    }

    let mut res = BeatMap{preempt: 0, cs: 0.0, od: res_od, track: res_track, objs: res_objs, hit_box: (1.0, 1.0)};
    res.set_ar(res_ar);
    res.set_cs(res_cs);

    return res;
}

