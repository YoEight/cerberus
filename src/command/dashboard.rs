use crate::common::{ CerberusResult, CerberusError };
use std::io;
use std::thread;
use std::time::Duration;
use std::sync::mpsc;
use tui::Terminal;
use tui::backend::{ Backend, TermionBackend };
use tui::widgets::{ Widget, Block, Borders };
use tui::layout::{ Layout, Constraint, Direction };
use termion::raw::IntoRawMode;
use termion::input::TermRead;
use termion::event::Key;

pub fn run(
    global: &clap::ArgMatches,
) -> CerberusResult<()> {
    run_dashboard(global).map_err(|e| {
        CerberusError::UserFault(
            format!("{}", e))
    })
}

struct App {
    should_quit: bool,
}

enum Msg {
    Input(Key),
    Tick,
}

fn draw<B>(
    terminal: &mut Terminal<B>,
    app: &App,
) -> io::Result<()>
    where
        B: Backend
{
    terminal.draw(|mut frame| {
        let size = frame.size();

        Block::default()
            .title("Cerberus")
            .borders(Borders::ALL)
            .render(&mut frame, size);
    })
}

fn run_dashboard(
    global: &clap::ArgMatches,
) -> io::Result<()> {
    let stdout = io::stdout().into_raw_mode()?;
    let backend = TermionBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;
    let tick_rate = Duration::from_millis(200);
    let mut app = App {
        should_quit: false,
    };

    terminal.clear()?;
    terminal.hide_cursor()?;

    let (tx, rx) = mpsc::channel();

    // Input thread.
    let input_handle = {
        let tx = tx.clone();

        thread::spawn(move || {
            let stdin = io::stdin();

            for evt in stdin.keys() {
                match evt {
                    Ok(key) => {
                        if let Err(_) = tx.send(Msg::Input(key)) {
                            return;
                        }
                    },

                    Err(_) => {}
                }
            }
        })
    };

    // Ticking thread.
    let tick_handle = {
        let tx = tx.clone();

        thread::spawn(move || {
            let tx = tx.clone();
            tx.send(Msg::Tick).unwrap();
            thread::sleep(tick_rate);
        })
    };

    loop {
        draw(&mut terminal, &app)?;

        match rx.recv().expect("Oopsie") {
            Msg::Input(key) => match key {
                Key::Char(c) => {
                    if c == 'q' {
                        app.should_quit = true ;
                    }
                },

                _ => {},
            },

            Msg::Tick => {
                // TODO - Refresh the UI.
            },
        }

        if app.should_quit {
            break;
        }
    }

    terminal.clear()?;

    Ok(())
}
