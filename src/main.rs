use crossterm::{
    event::{self, KeyCode, KeyEventKind, MouseEventKind, MouseEvent, Event, EnableMouseCapture, DisableMouseCapture},
    terminal::{
        disable_raw_mode, enable_raw_mode, EnterAlternateScreen,
        LeaveAlternateScreen,
    },
    ExecutableCommand
};
use ratatui::{
    style::{Color, Stylize},
    prelude::{CrosstermBackend, Terminal, Marker, symbols::scrollbar, Line, Layout, Direction, Constraint, Style},
    widgets::{canvas::*, *, BorderType, Borders}, Frame, symbols::line,
};
use std::{io::{stdout, Result, Stdout}, u128, path::PathBuf, ffi::OsStr, env::home_dir};
use std::time::Instant;
use std::{path::Path, fs::File};
use std::io::{self, BufRead, BufReader};
use rodio::{Decoder, OutputStream, source::Source};

pub mod beat_map; 
use beat_map::BeatMap;

fn menu_loop(terminal: &mut Terminal<CrosstermBackend<Stdout>>, config: Config) -> Result<()>{
    let mut maps = Vec::<PathBuf>::new();
    find_maps(config.songs.as_path(), &mut maps);

    let mut text = Vec::<Line>::new();
    for map in &maps {
        let wow = String::from(map.to_str().unwrap_or("fail"));
        text.push(Line::from(wow.clone()));
    }
 
    for line in &mut text {
        line.patch_style(Style::default().light_blue().on_black());
    }
    text[0].patch_style(Style::default().black().on_light_blue());

    let mut scroll: usize = 0;
    let mut scroll_state = ScrollbarState::new(maps.len());
    loop {
        terminal.draw(|frame| {
            let area = frame.size();
            let par =  Paragraph::new(text.clone())
                .gray()
                .on_black()
                .block(Block::default().title("maps").borders(Borders::ALL).border_type(BorderType::Rounded))
                .scroll((scroll as u16, 0));

             /*let main_layout = Layout::default()
                .direction(Direction::Horizontal)
                .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
                .split(frame.size());

            let right_layout = Layout::default()
                .direction(Direction::Vertical)
                .constraints([Constraint::Percentage(20), Constraint::Percentage(60), Constraint::Percentage(20)])
                .split(main_layout[1]);
            */

            frame.render_widget(par, area);
            frame.render_stateful_widget(
                Scrollbar::default()
                    .orientation(ScrollbarOrientation::VerticalRight)
                    .symbols(scrollbar::VERTICAL)
                    .begin_symbol(None)
                    .track_symbol(None)
                    .end_symbol(None),
                area,
                &mut scroll_state,
            );
        })?;

        if event::poll(std::time::Duration::from_millis(16))? {
            match event::read()? {
                event::Event::Key(key) => {
                    if key.kind != event::KeyEventKind::Press {
                        continue;
                    }

                    if key.code ==KeyCode::Enter {
                        BeatMap::play_map(maps[scroll].as_path(), terminal)?;
                    }

                    if key.code == KeyCode::Down && scroll < maps.len()-1 {
                        text[scroll].patch_style(Style::default().light_blue().on_black());
                        scroll += 1;
                        text[scroll].patch_style(Style::default().black().on_light_blue());
                        scroll_state = scroll_state.position(scroll);
                    }

                    if key.code == KeyCode::Up && scroll > 0 {
                        text[scroll].patch_style(Style::default().light_blue().on_black());
                        scroll -= 1;
                        text[scroll].patch_style(Style::default().black().on_light_blue());
                        scroll_state = scroll_state.position(scroll);
                    }

                    if key.code == KeyCode::Char('q') || key.code == KeyCode::Char('Q') {
                        break;
                    }

                },
                event::Event::Mouse(mouse) => {                        
                    if mouse.kind == event::MouseEventKind::Moved {
                    }
                },
                _ => (),
            }
        }
    }
    Ok(())
}

fn find_maps(path: &Path, res: &mut Vec<PathBuf>) {
    if !path.is_dir() {
        return;
    }
    for entry in path.read_dir().expect("read_dir call failed") {
        if let Ok(entry) = entry {
            find_maps(entry.path().as_path(), res);
            if entry.path().extension().unwrap_or(&OsStr::new("none")) == OsStr::new("osu") {
                res.push(entry.path());
            }
        }
    }
}

struct Config{
    songs: PathBuf,
}

fn read_config() -> Config {
    let file = File::open(home_dir().unwrap().join(Path::new(".config/tosu/init")).as_path()).unwrap();
    let lines = io::BufReader::new(file).lines();
    let mut res_songs = PathBuf::new();

    for line in lines {
        let line = line.unwrap();
        
        if line.starts_with("Songs") {
            let temp: Vec<&str> = line.split(":").collect(); 
            res_songs = Path::new(temp[1]).to_path_buf();
        }
    }
    Config {songs: res_songs}
}

fn main() -> Result<()> {
    let config = read_config();

    stdout().execute(EnterAlternateScreen)?;
    stdout().execute(EnableMouseCapture)?;
    enable_raw_mode()?;
    let mut terminal = Terminal::new(CrosstermBackend::new(stdout()))?;
    terminal.clear()?;

    menu_loop(&mut terminal, config)?;

    stdout().execute(LeaveAlternateScreen)?;
    stdout().execute(DisableMouseCapture)?;
    disable_raw_mode()?;
    
    Ok(())
}
