use std::sync::{atomic::Ordering, Arc};

use baseview::{gl::GlConfig, Size, WindowHandle, WindowOpenOptions, WindowScalePolicy};
use bevy::{app::App, render::view::window};
use crossbeam::atomic::AtomicCell;
use nih_plug::editor::Editor;

use crate::{param_plugin::{NIHContext, NIHCurrentParam, NIHParamPlugin}, BevyState};

pub(crate) struct BevyEditor {
    pub(crate) bevy_state: Arc<BevyState>,

    /// The user's build function. Applied once at the start of the application.
    pub(crate) build: Arc<dyn Fn(&mut App) -> &mut App + 'static + Send + Sync>,

    /// The scaling factor reported by the host, if any. On macOS this will never be set and we
    /// should use the system scaling factor instead.
    pub(crate) scaling_factor: AtomicCell<Option<f32>>,
}

impl Editor for BevyEditor {
    fn spawn(
        &self,
        parent: nih_plug::prelude::ParentWindowHandle,
        context: Arc<dyn nih_plug::prelude::GuiContext>,
    ) -> Box<dyn std::any::Any + Send> {
        let build = self.build.clone();

        let (unscaled_width, unscaled_height) = self.bevy_state.size();
        let scaling_factor = self.scaling_factor.load();

        let window = bevy_baseview::open_parented(
            parent, 
            WindowOpenOptions {
                title: String::from("bevy window"),
                // Baseview should be doing the DPI scaling for us
                size: Size::new(unscaled_width as f64, unscaled_height as f64),
                // NOTE: For some reason passing 1.0 here causes the UI to be scaled on macOS but
                //       not the mouse events.
                scale: scaling_factor
                    .map(|factor| WindowScalePolicy::ScaleFactor(factor as f64))
                    .unwrap_or(WindowScalePolicy::SystemScaleFactor),
                #[cfg(feature = "opengl")]
                gl_config: Some(GlConfig {
                    version: (3, 2),
                    red_bits: 8,
                    blue_bits: 8,
                    green_bits: 8,
                    alpha_bits: 8,
                    depth_bits: 24,
                    stencil_bits: 8,
                    samples: None,
                    srgb: true,
                    double_buffer: true,
                    vsync: true,
                    ..Default::default()
                }),
            },
            move |app| {
                app.insert_resource(NIHContext::new(context.clone()))
                    .init_resource::<NIHCurrentParam>()
                    .add_plugins(NIHParamPlugin);
                build(app)
            }
        );

        Box::new(BevyEditorHandle {
            bevy_state: self.bevy_state.clone(),
            window,
        })
    }

    fn size(&self) -> (u32, u32) {
        self.bevy_state.size()
    }

    fn set_scale_factor(&self, factor: f32) -> bool {
        if self.bevy_state.is_open() {
            return false;
        }

        self.scaling_factor.store(Some(factor));
        true
    }

    // For the moment we redraw every frame :p
    // Potential TODO in the future to optimize the performance
    // Then again Knuth said that "Premature optimization is the root of all evil"
    // But my code will never mature. It'll stay young forever -- eternal and unoptimized.
    fn param_value_changed(&self, _id: &str, _normalized_value: f32) {}

    fn param_modulation_changed(&self, _id: &str, _modulation_offset: f32) {}

    fn param_values_changed(&self) {}
}

/// The window handle used for [`BevyEditor`].
struct BevyEditorHandle {
    bevy_state: Arc<BevyState>,
    window: WindowHandle,
}

/// The window handle enum stored within 'WindowHandle' contains raw pointers. Is there a way around
/// having this requirement?
unsafe impl Send for BevyEditorHandle {}

impl Drop for BevyEditorHandle {
    fn drop(&mut self) {
        self.bevy_state.open.store(false, Ordering::Release);
        // XXX: This should automatically happen when the handle gets dropped, but apparently not
        self.window.close();
    }
}