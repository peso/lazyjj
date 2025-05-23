//! The event module is where events is generated. This means
//! capture keyboard and mouse events from user, as well as listening
//! for file system notifications in case somebody used jj on this
//! repository.

use std::{
    time::{
        Duration,
        Instant,
    },
    path::PathBuf,
    sync::mpsc,
};

use notify::{RecursiveMode, Watcher};
use ratatui::crossterm;
use tracing::debug;

/// Minimum time between notify events to the app
const NOTIFY_COOL_DOWN: Duration = Duration::from_secs(1);

/// Input event to the app
pub enum AppEvent {
    /// Keyboard or mouse input from user
    UserInput(crossterm::event::Event),
    /// The .jj folder was touched, so app must redraw
    DirtyJj,
}

type NotifyEvent = Result<notify::Event, notify::Error>;

/// Generator of events to the app
pub struct EventSource {
    repo_watcher: Option<notify::RecommendedWatcher>,
    repo_channel: Option<mpsc::Receiver<NotifyEvent>>,
    repo_last_notice: Instant,
    last_event_none: bool,

}

impl EventSource {
    pub fn new() -> Self {
        Self {
            repo_watcher: None,
            repo_channel: None,
            repo_last_notice: Instant::now(),
            last_event_none: false,
        }   
    }

    /// Launch a file system watcher as event source
    pub fn launch_watcher(&mut self, jj_folder: PathBuf) {
        // The notify watcher uses a channel to send file system events
        let (tx, rx) = mpsc::channel();

        // Create a watcher attached to the channel
        let watch_result = notify::recommended_watcher(tx);
        let Ok(mut watcher) = watch_result
        else {
            let Err(e) = watch_result else { unreachable!(); };
            eprintln!("Failed to create notify watcher: {:?}", e);
            return;
        };

        // Start watching
        if let Err(e) = watcher.watch(&jj_folder, RecursiveMode::Recursive) {
            eprintln!("Failed to watch .jj directory: {:?}", e);
            return;
        }

        // Commit to listening on watcher
        self.repo_watcher = Some(watcher);
        self.repo_channel = Some(rx);
    }

    /// Receive an AppEvent if one is waiting
    pub fn try_recv(&mut self) -> Option<AppEvent> {
        // Makes the loop wait 1 second
        // for user events if nothing happens. This will
        // reduce cpu load and make the app discover notify
        // events after <1 second delay.
        let poll_period = if self.last_event_none {
            Duration::from_secs(1)
        } else {
            Duration::ZERO
        };

        // Check for keyboard event
        if crossterm::event::poll(poll_period).unwrap() {
            let event = crossterm::event::read().unwrap();
            self.last_event_none = false;
            debug!("Keyboard event");
            return Some(AppEvent::UserInput(event));
        };

        // Check for notify event
        if let Some(ref notify_reciever) = self.repo_channel {
            if let Ok(_) = notify_reciever.try_recv() {
                // drain all events
                while notify_reciever.try_recv().is_ok() {
                }
                // Cool down period
                if self.repo_last_notice.elapsed() > NOTIFY_COOL_DOWN {
                    self.repo_last_notice = Instant::now();
                    self.last_event_none = false;
                    debug!("Notify event");
                    return Some(AppEvent::DirtyJj)
                }
            }
        }

        // No event found
        self.last_event_none = true;
        debug!("No event");
        None
    }
}