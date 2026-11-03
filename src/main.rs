mod agent;
mod ai;
mod app;
mod cache;
mod chat;
mod encode;
mod events;
mod exec;
mod fuzzy;
mod git;
mod github;
mod history;
#[cfg(test)]
mod history_test;
mod keys;
mod log_parse;
mod nav;
mod porcelain;
mod prompt;
mod risk;
mod scan;
mod stats;
mod theme;
mod tools;
mod ui;
mod ui_activity;
mod ui_bar;
mod ui_changes;
mod ui_chat;
mod ui_detail;
mod ui_feed;
mod ui_folder;
mod ui_heat;
mod ui_help;
mod ui_list;
mod ui_modal;
mod ui_mosaic;
mod ui_people;
mod ui_row;
mod wrap;

use app::App;
use keys::Exit;
use ratatui::{
    Terminal,
    crossterm::{
        event::{self, Event, KeyEventKind},
        execute,
        terminal::{EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode},
    },
    prelude::CrosstermBackend,
};
use scan::Msg;
use std::{
    io::{self, Stderr},
    path::PathBuf,
    sync::mpsc::{self, Receiver, Sender},
    time::{Duration, Instant},
};

type Term = Terminal<CrosstermBackend<Stderr>>;

fn main() -> io::Result<()> {
    let root = std::path::absolute(std::env::args().nth(1).map_or(std::env::current_dir()?, PathBuf::from))?;
    if !root.is_dir() {
        return Err(io::Error::new(
            io::ErrorKind::NotFound,
            format!("not a directory: {}", root.display()),
        ));
    }
    let (tx, rx) = mpsc::channel();
    scan::spawn_scan(root.clone(), tx.clone());

    // TUI renders on stderr so stdout carries only the chosen path: `cd "$(gitnav ~/dev)"`.
    enable_raw_mode()?;
    execute!(io::stderr(), EnterAlternateScreen)?;
    let hook = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |p| {
        restore();
        hook(p)
    }));
    let mut app = App::new(root);
    let res = Terminal::new(CrosstermBackend::new(io::stderr())).and_then(|mut t| run(&mut t, &mut app, rx, tx));
    restore();
    cache::save(&app.root, &app.repos, &app.pulses);
    if let Some(p) = res? {
        println!("{}", p.display());
    }
    Ok(())
}

fn restore() {
    let _ = disable_raw_mode();
    let _ = execute!(io::stderr(), LeaveAlternateScreen);
}

const FRAME: Duration = Duration::from_millis(80);

fn run(t: &mut Term, app: &mut App, rx: Receiver<Msg>, tx: Sender<Msg>) -> io::Result<Option<PathBuf>> {
    // Draw on change, plus spinner frames only while work is in flight.
    let (mut redraw, mut last) = (true, Instant::now());
    loop {
        for m in rx.try_iter() {
            app.apply(m);
            redraw = true;
        }
        let animating =
            app.scanning || app.chat.busy || !app.busy.is_empty() || app.repos.values().any(Option::is_none);
        if animating && last.elapsed() >= FRAME {
            app.tick += 1;
            redraw = true;
        }
        if redraw {
            app.want_info(&tx);
            t.draw(|f| app.draw(f))?;
            (redraw, last) = (false, Instant::now());
        }
        if !event::poll(Duration::from_millis(30))? {
            continue;
        }
        redraw = true;
        let Event::Key(k) = event::read()? else {
            continue;
        };
        if k.kind != KeyEventKind::Press {
            continue;
        }
        match app.key(k, &tx) {
            Some(Exit::Quit) => return Ok(None),
            Some(Exit::Cd(p)) => return Ok(Some(p)),
            None => {}
        }
    }
}
