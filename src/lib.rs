use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use bevy::app::App;
use crossbeam::atomic::AtomicCell;
use nih_plug::editor::Editor;
use serde::{Serialize, Deserialize};

use nih_plug::params::persist::PersistentField;

mod editor;
pub mod param_plugin;

pub fn create_bevy_editor<B>(
    bevy_state: Arc<BevyState>,
    build: B,
) -> Option<Box<dyn Editor>>
where
    B: Fn(&mut App) -> &mut App + 'static + Send + Sync,
{
    Some(Box::new(editor::BevyEditor {
        bevy_state,
        build: Arc::new(build),

        // TODO: We can't get the size of the window when baseview does its own scaling, so if the
        //       host does not set a scale factor on Windows or Linux we should just use a factor of
        //       1. That may make the GUI tiny but it also prevents it from getting cut off.
        #[cfg(target_os = "macos")]
        scaling_factor: AtomicCell::new(None),
        #[cfg(not(target_os = "macos"))]
        scaling_factor: AtomicCell::new(Some(1.0)),
    }))
}

#[derive(Debug, Serialize, Deserialize)]
pub struct BevyState {
    #[serde(with = "nih_plug::params::persist::serialize_atomic_cell")]
    size: AtomicCell<(u32, u32)>,
    #[serde(skip)]
    open: AtomicBool,
}

impl<'a> PersistentField<'a, BevyState> for Arc<BevyState> {
    fn set(&self, new_value: BevyState) {
        self.size.store(new_value.size.load());
    }

    fn map<F, R>(&self, f: F) -> R
    where
        F: Fn(&BevyState) -> R,
    {
        f(self)
    }
}

impl BevyState {
    /// Initialize the GUI's state. This value can be passed to [`create_iced_editor()`]. The window
    /// size is in logical pixels, so before it is multiplied by the DPI scaling factor.
    pub fn from_size(width: u32, height: u32) -> Arc<BevyState> {
        Arc::new(BevyState {
            size: AtomicCell::new((width, height)),
            open: AtomicBool::new(false),
        })
    }

    /// Returns a `(width, height)` pair for the current size of the GUI in logical pixels.
    pub fn size(&self) -> (u32, u32) {
        self.size.load()
    }

    /// Whether the GUI is currently visible.
    // Called `is_open()` instead of `open()` to avoid the ambiguity.
    pub fn is_open(&self) -> bool {
        self.open.load(Ordering::Acquire)
    }
}