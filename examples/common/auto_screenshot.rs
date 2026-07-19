//! One-shot window screenshot when `BEVY_REACT_SCREENSHOT` is set.
//!
//! Used by example capture scripts — inactive unless the env var is present.
//! Optional `BEVY_REACT_SCREENSHOT_DELAY` (seconds, default 3.0) waits for UI mount.

use std::path::PathBuf;

use bevy::app::AppExit;
use bevy::prelude::*;
use bevy::render::view::screenshot::{save_to_disk, Screenshot};

pub struct AutoScreenshotPlugin;

impl Plugin for AutoScreenshotPlugin {
    fn build(&self, app: &mut App) {
        if std::env::var_os("BEVY_REACT_SCREENSHOT").is_none() {
            return;
        }
        app.init_resource::<ScreenshotState>()
            .add_systems(Update, auto_screenshot_system);
    }
}

#[derive(Resource, Default)]
enum ScreenshotState {
    #[default]
    Waiting,
    Capturing {
        path: PathBuf,
        started_at: f32,
    },
}

fn auto_screenshot_system(
    mut commands: Commands,
    time: Res<Time>,
    mut state: ResMut<ScreenshotState>,
    mut exit: MessageWriter<AppExit>,
) {
    let Ok(path) = std::env::var("BEVY_REACT_SCREENSHOT") else {
        return;
    };
    let delay: f32 = std::env::var("BEVY_REACT_SCREENSHOT_DELAY")
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(3.0);

    match &*state {
        ScreenshotState::Waiting => {
            if time.elapsed_secs() < delay {
                return;
            }
            let path_buf = PathBuf::from(&path);
            if let Some(parent) = path_buf.parent() {
                let _ = std::fs::create_dir_all(parent);
            }
            info!(
                "Capturing screenshot → {} (after {delay:.1}s)",
                path_buf.display()
            );
            commands
                .spawn(Screenshot::primary_window())
                .observe(save_to_disk(path_buf.clone()));
            *state = ScreenshotState::Capturing {
                path: path_buf,
                started_at: time.elapsed_secs(),
            };
        }
        ScreenshotState::Capturing { path, started_at } => {
            let elapsed = time.elapsed_secs() - *started_at;
            if path.is_file() && elapsed > 0.25 {
                info!("Screenshot ready at {}; exiting", path.display());
                exit.write(AppExit::Success);
            } else if elapsed > 15.0 {
                error!(
                    "Timed out waiting for screenshot at {}",
                    path.display()
                );
                exit.write(AppExit::error());
            }
        }
    }
}
