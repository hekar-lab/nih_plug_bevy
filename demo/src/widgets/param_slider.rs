use std::ops::DerefMut;

use bevy::{input::mouse::MouseScrollUnit, prelude::*, ui::RelativeCursorPosition};

use nih_plug::{params::Param, prelude::ParamPtr};
use nih_plug_bevy::param_plugin::{NIHParam, ParamAction, ParamEvent};
use sickle_ui_scaffold::{prelude::*, ui_commands::UpdateTextExt};

use sickle_ui::widgets::layout::{
    container::UiContainerExt,
    label::{LabelConfig, UiLabelExt},
};

pub struct ParamSliderPlugin;

impl Plugin for ParamSliderPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(ComponentThemePlugin::<ParamSlider>::default())
            .add_systems(
                Update,
                (
                    update_slider_on_scroll.after(ScrollableUpdate),
                    update_slider_on_drag.after(DraggableUpdate),
                    update_slider_on_bar_change,
                    update_slider_handle,
                    update_slider_readout,
                )
                    .chain(),
            );
    }
}

// TODO: Add input for value (w/ read/write flags)
// TODO: Support click-on-bar value setting
fn update_slider_on_scroll(
    q_scrollables: Query<
        (AnyOf<(&ParamSliderBar, &ParamSliderDragHandle)>, &Scrollable),
        Changed<Scrollable>,
    >,
    mut q_slider: Query<(Entity, &mut ParamSlider)>,
    mut param_evt: EventWriter<ParamEvent>,
) {
    for ((slider_bar, handle), scrollable) in &q_scrollables {
        let Some((axis, diff, unit)) = scrollable.last_change() else {
            continue;
        };
        if axis == ScrollAxis::Horizontal {
            continue;
        }

        let slider_id = if let Some(slider_bar) = slider_bar {
            slider_bar.slider
        } else if let Some(handle) = handle {
            handle.slider
        } else {
            continue;
        };

        let Ok((slider_id, mut slider)) = q_slider.get_mut(slider_id) else {
            continue;
        };

        let offset = match unit {
            MouseScrollUnit::Line => -diff * 5.,
            MouseScrollUnit::Pixel => -diff,
        };

        let fraction = offset / 100.;
        slider.ratio = (slider.ratio + fraction).clamp(0., 1.);

        param_evt.send(ParamEvent::begin(slider_id));
        param_evt.send(ParamEvent::set(slider_id, slider.ratio));
        param_evt.send(ParamEvent::end(slider_id));
    }
}

fn update_slider_on_drag(
    q_draggable: Query<(&Draggable, &ParamSliderDragHandle, &Node), Changed<Draggable>>,
    q_node: Query<&Node>,
    mut q_slider: Query<(Entity, &mut ParamSlider)>,
    mut param_evt: EventWriter<ParamEvent>,
) {
    for (draggable, handle, node) in &q_draggable {
        let Ok((slider_id, mut slider)) = q_slider.get_mut(handle.slider) else {
            continue;
        };

        if draggable.state == DragState::Inactive || draggable.state == DragState::MaybeDragged {
            continue;
        }

        if draggable.state == DragState::DragCanceled {
            if let Some(base_ratio) = slider.base_ratio {
                slider.ratio = base_ratio;
                param_evt.send(ParamEvent::set(slider_id, slider.ratio));
                param_evt.send(ParamEvent::end(slider_id));
                continue;
            }
        }

        if draggable.state == DragState::DragStart {
            slider.base_ratio = slider.ratio.into();
            param_evt.send(ParamEvent::begin(slider_id));
        }

        let Ok(slider_bar) = q_node.get(slider.bar_container) else {
            continue;
        };
        let Some(diff) = draggable.diff else {
            continue;
        };

        let axis = &slider.config.axis;
        let fraction = match axis {
            ParamSliderAxis::Horizontal => {
                let width = slider_bar.size().x - node.size().x;
                if diff.x == 0. || width == 0. {
                    continue;
                }
                diff.x / width
            }
            ParamSliderAxis::Vertical => {
                let height = slider_bar.size().y - node.size().y;
                if diff.y == 0. || height == 0. {
                    continue;
                }
                -diff.y / height
            }
        };

        slider.ratio = (slider.ratio + fraction).clamp(0., 1.);
        param_evt.send(ParamEvent::set(slider_id, slider.ratio));

        if draggable.state == DragState::DragEnd {
            param_evt.send(ParamEvent::end(slider_id));
        }
    }
}

fn update_slider_on_bar_change(
    q_slider_bars: Query<&ParamSliderBar, Changed<Node>>,
    mut q_slider: Query<&mut ParamSlider>,
) {
    for bar in &q_slider_bars {
        let Ok(mut slider) = q_slider.get_mut(bar.slider) else {
            continue;
        };

        slider.deref_mut();
    }
}

fn update_slider_handle(
    q_slider: Query<&ParamSlider, Or<(Changed<ParamSlider>, Changed<Node>)>>,
    q_node: Query<&Node>,
    mut q_hadle_style: Query<(&Node, &mut Style), With<ParamSliderDragHandle>>,
) {
    for slider in &q_slider {
        let Ok(slider_bar) = q_node.get(slider.bar_container) else {
            continue;
        };
        let Ok((node, mut style)) = q_hadle_style.get_mut(slider.handle) else {
            continue;
        };

        let axis = &slider.config.axis;
        match axis {
            ParamSliderAxis::Horizontal => {
                let width = slider_bar.size().x - node.size().x;
                let handle_position = width * slider.ratio;
                if style.left != Val::Px(handle_position) {
                    style.left = Val::Px(handle_position);
                }
            }
            ParamSliderAxis::Vertical => {
                let height = slider_bar.size().y - node.size().y;
                let handle_position = height * (1. - slider.ratio);
                if style.top != Val::Px(handle_position) {
                    style.top = Val::Px(handle_position);
                }
            }
        }
    }
}

fn update_slider_readout(q_slider: Query<&ParamSlider, Changed<ParamSlider>>, mut commands: Commands) {
    for slider in &q_slider {
        if !slider.config.show_current {
            continue;
        }

        commands
            .entity(slider.readout)
            .update_text(format!("{:.1}", slider.value()));
    }
}

#[derive(Copy, Clone, Debug, Default, Eq, PartialEq, Reflect)]
pub enum ParamSliderAxis {
    #[default]
    Horizontal,
    Vertical,
}

#[derive(Component, Debug, Reflect)]
#[reflect(Component)]
pub struct ParamSliderDragHandle {
    pub slider: Entity,
}

impl Default for ParamSliderDragHandle {
    fn default() -> Self {
        Self {
            slider: Entity::PLACEHOLDER,
        }
    }
}

#[derive(Component, Debug, Reflect)]
#[reflect(Component)]
pub struct ParamSliderBar {
    pub slider: Entity,
}

impl Default for ParamSliderBar {
    fn default() -> Self {
        Self {
            slider: Entity::PLACEHOLDER,
        }
    }
}

#[derive(Component, Clone, Debug, Reflect)]
pub struct ParamSliderConfig {
    pub label: Option<String>,
    pub min: f32,
    pub max: f32,
    pub show_current: bool,
    pub axis: ParamSliderAxis,
}

impl ParamSliderConfig {
    pub fn new(
        label: impl Into<Option<String>>,
        min: f32,
        max: f32,
        show_current: bool,
        axis: ParamSliderAxis,
    ) -> Self {
        ParamSliderConfig {
            label: label.into(),
            min,
            max,
            show_current,
            axis,
        }
    }

    pub fn horizontal(
        label: impl Into<Option<String>>,
        min: f32,
        max: f32,
        show_current: bool,
    ) -> Self {
        Self::new(
            label.into(),
            min,
            max,
            show_current,
            ParamSliderAxis::Horizontal,
        )
    }

    pub fn vertical(
        label: impl Into<Option<String>>,
        min: f32,
        max: f32,
        show_current: bool,
    ) -> Self {
        Self::new(
            label.into(),
            min,
            max,
            show_current,
            ParamSliderAxis::Vertical,
        )
    }

    // pub fn with_value(self, value: f32) -> Self {
    //     if value >= self.min && value <= self.max {
    //         return Self {
    //             initial_value: value,
    //             ..self
    //         };
    //     }

    //     panic!("Value must be between min and max!");
    // }
}

impl Default for ParamSliderConfig {
    fn default() -> Self {
        Self {
            label: None,
            min: 0.,
            max: 1.,
            show_current: Default::default(),
            axis: Default::default(),
        }
    }
}

#[derive(Component, Debug, Reflect)]
#[reflect(Component)]
pub struct ParamSlider {
    ratio: f32,
    config: ParamSliderConfig,
    label: Entity,
    bar_container: Entity,
    bar: Entity,
    handle: Entity,
    readout_container: Entity,
    readout: Entity,
    base_ratio: Option<f32>,
}

impl Default for ParamSlider {
    fn default() -> Self {
        Self {
            ratio: Default::default(),
            config: Default::default(),
            base_ratio: None,
            label: Entity::PLACEHOLDER,
            bar_container: Entity::PLACEHOLDER,
            bar: Entity::PLACEHOLDER,
            handle: Entity::PLACEHOLDER,
            readout_container: Entity::PLACEHOLDER,
            readout: Entity::PLACEHOLDER,
        }
    }
}

impl UiContext for ParamSlider {
    fn get(&self, target: &str) -> Result<Entity, String> {
        match target {
            ParamSlider::LABEL => Ok(self.label),
            ParamSlider::BAR_CONTAINER => Ok(self.bar_container),
            ParamSlider::BAR => Ok(self.bar),
            ParamSlider::HANDLE => Ok(self.handle),
            ParamSlider::READOUT_CONTAINER => Ok(self.readout_container),
            ParamSlider::READOUT => Ok(self.readout),
            _ => Err(format!(
                "{} doesn't exist for Slider. Possible contexts: {:?}",
                target,
                self.contexts()
            )),
        }
    }

    fn contexts(&self) -> Vec<&'static str> {
        vec![
            ParamSlider::LABEL,
            ParamSlider::BAR_CONTAINER,
            ParamSlider::BAR,
            ParamSlider::HANDLE,
            ParamSlider::READOUT_CONTAINER,
            ParamSlider::READOUT,
        ]
    }
}

impl DefaultTheme for ParamSlider {
    fn default_theme() -> Option<Theme<ParamSlider>> {
        ParamSlider::theme().into()
    }
}

impl ParamSlider {
    pub const LABEL: &'static str = "Label";
    pub const BAR_CONTAINER: &'static str = "BarContainer";
    pub const BAR: &'static str = "Bar";
    pub const HANDLE: &'static str = "Handle";
    pub const READOUT_CONTAINER: &'static str = "ReadoutContainer";
    pub const READOUT: &'static str = "Readout";

    pub fn value(&self) -> f32 {
        self.config.min.lerp(self.config.max, self.ratio)
    }

    pub fn config(&self) -> &ParamSliderConfig {
        &self.config
    }

    pub fn set_value(&mut self, value: f32) {
        if value > self.config.max || value < self.config.min {
            warn!("Tried to set slider value outside of range");
            return;
        }

        self.ratio = (value - self.config.min) / (self.config.max + (0. - self.config.min))
    }

    pub fn theme() -> Theme<ParamSlider> {
        let base_theme = PseudoTheme::deferred_context(None, ParamSlider::primary_style);
        Theme::new(vec![base_theme])
    }

    fn primary_style(style_builder: &mut StyleBuilder, slider: &ParamSlider, theme_data: &ThemeData) {
        let theme_spacing = theme_data.spacing;
        let colors = theme_data.colors();
        let font = theme_data
            .text
            .get(FontStyle::Body, FontScale::Medium, FontType::Regular);

        match slider.config().axis {
            ParamSliderAxis::Horizontal => {
                style_builder
                    .justify_content(JustifyContent::SpaceBetween)
                    .align_items(AlignItems::Center)
                    .width(Val::Percent(100.))
                    .height(Val::Px(theme_spacing.areas.small))
                    .padding(UiRect::horizontal(Val::Px(theme_spacing.gaps.medium)));

                style_builder
                    .switch_target(ParamSlider::LABEL)
                    .margin(UiRect::right(Val::Px(theme_spacing.gaps.medium)));

                style_builder
                    .switch_target(ParamSlider::BAR_CONTAINER)
                    .width(Val::Percent(100.));

                style_builder
                    .switch_target(ParamSlider::BAR)
                    .width(Val::Percent(100.))
                    .height(Val::Px(theme_spacing.gaps.small))
                    .margin(UiRect::vertical(Val::Px(theme_spacing.gaps.medium)));

                style_builder
                    .switch_target(ParamSlider::READOUT)
                    .min_width(Val::Px(theme_spacing.areas.medium))
                    .margin(UiRect::left(Val::Px(theme_spacing.gaps.medium)));

                style_builder
                    .switch_context(ParamSlider::HANDLE, None)
                    .margin(UiRect::top(Val::Px(
                        -theme_spacing.gaps.medium + theme_spacing.borders.extra_small,
                    )));
            }
            ParamSliderAxis::Vertical => {
                style_builder
                    .flex_direction(FlexDirection::ColumnReverse)
                    .justify_content(JustifyContent::SpaceBetween)
                    .align_items(AlignItems::Center)
                    .height(Val::Percent(100.))
                    .padding(UiRect::vertical(Val::Px(theme_spacing.gaps.medium)));

                style_builder
                    .switch_target(ParamSlider::LABEL)
                    .margin(UiRect::px(
                        theme_spacing.gaps.small,
                        theme_spacing.gaps.small,
                        theme_spacing.gaps.small,
                        0.,
                    ));

                style_builder
                    .switch_target(ParamSlider::BAR_CONTAINER)
                    .flex_direction(FlexDirection::Column)
                    .height(Val::Percent(100.));

                style_builder
                    .switch_target(ParamSlider::BAR)
                    .flex_direction(FlexDirection::Column)
                    .width(Val::Px(theme_spacing.gaps.small))
                    .height(Val::Percent(100.))
                    .margin(UiRect::horizontal(Val::Px(theme_spacing.gaps.medium)));

                style_builder
                    .switch_target(ParamSlider::READOUT_CONTAINER)
                    .justify_content(JustifyContent::Center)
                    .justify_items(JustifyItems::Center)
                    .width(Val::Px(theme_spacing.areas.medium))
                    .overflow(Overflow::clip());

                style_builder
                    .switch_target(ParamSlider::READOUT)
                    .margin(UiRect::all(Val::Px(theme_spacing.gaps.small)));

                style_builder
                    .switch_context(ParamSlider::HANDLE, None)
                    .margin(UiRect::left(Val::Px(
                        -theme_spacing.gaps.medium + theme_spacing.borders.extra_small,
                    )));
            }
        }

        style_builder.reset_context();

        style_builder
            .switch_target(ParamSlider::LABEL)
            .sized_font(font.clone())
            .font_color(colors.on(On::Surface));

        if slider.config().label.is_none() {
            style_builder
                .switch_target(ParamSlider::LABEL)
                .display(Display::None)
                .visibility(Visibility::Hidden);
        } else {
            style_builder
                .switch_target(ParamSlider::LABEL)
                .display(Display::Flex)
                .visibility(Visibility::Inherited);
        }

        if !slider.config().show_current {
            style_builder
                .switch_target(ParamSlider::READOUT_CONTAINER)
                .display(Display::None)
                .visibility(Visibility::Hidden);
        } else {
            style_builder
                .switch_target(ParamSlider::READOUT_CONTAINER)
                .display(Display::Flex)
                .visibility(Visibility::Inherited);
        }

        style_builder
            .switch_target(ParamSlider::READOUT)
            .sized_font(font.clone())
            .font_color(colors.on(On::Surface));

        style_builder
            .switch_target(ParamSlider::BAR)
            .border(UiRect::px(
                0.,
                theme_spacing.borders.extra_small,
                0.,
                theme_spacing.borders.extra_small,
            ))
            .background_color(colors.surface(Surface::SurfaceVariant))
            .border_color(colors.accent(Accent::Shadow));

        style_builder
            .switch_context(ParamSlider::HANDLE, None)
            .size(Val::Px(theme_spacing.icons.small))
            .border(UiRect::all(Val::Px(theme_spacing.borders.extra_small)))
            .border_color(colors.accent(Accent::Shadow))
            .border_radius(BorderRadius::all(Val::Px(theme_spacing.icons.small)))
            .animated()
            .background_color(AnimatedVals {
                idle: colors.accent(Accent::Primary),
                hover: colors.container(Container::Primary).into(),
                ..default()
            })
            .copy_from(theme_data.interaction_animation);
    }

    fn container(name: String) -> impl Bundle {
        (Name::new(name), NodeBundle::default())
    }

    fn bar_container() -> impl Bundle {
        (
            Name::new("Bar Container"),
            NodeBundle::default(),
            Interaction::default(),
            Scrollable::default(),
        )
    }

    fn bar() -> impl Bundle {
        (Name::new("Slider Bar"), NodeBundle::default())
    }

    fn handle(slider: Entity) -> impl Bundle {
        (
            Name::new("Handle"),
            ButtonBundle::default(),
            TrackedInteraction::default(),
            ParamSliderDragHandle { slider },
            Draggable::default(),
            RelativeCursorPosition::default(),
            Scrollable::default(),
        )
    }

    fn readout_container() -> impl Bundle {
        (Name::new("Readout"), NodeBundle::default())
    }
}

pub trait UiParamSliderExt {
    fn param_slider(&mut self, config: ParamSliderConfig, param: ParamPtr) -> UiBuilder<Entity>;
}

impl UiParamSliderExt for UiBuilder<'_, Entity> {
    fn param_slider(&mut self, config: ParamSliderConfig, param: ParamPtr) -> UiBuilder<Entity> {
        let mut slider = ParamSlider {
            ratio: unsafe { param.default_normalized_value() },
            config: config.clone(),
            ..default()
        };

        match param {
            ParamPtr::FloatParam(_) => {},
            _ => { panic!("Parameter type not supported by slider") }
        }
        let nih_param = NIHParam::new(param);

        let label = match config.label {
            Some(label) => label,
            None => "".into(),
        };
        let has_label = label.len() > 0;
        let name = match has_label {
            true => format!("Slider [{}]", label.clone()),
            false => "Slider".into(),
        };

        let mut input = self.container(ParamSlider::container(name), |container| {
            let input_id = container.id();

            slider.label = container.label(LabelConfig { label, ..default() }).id();
            slider.bar_container = container
                .container(
                    (ParamSlider::bar_container(), ParamSliderBar { slider: input_id }),
                    |bar_container| {
                        slider.bar = bar_container
                            .container(ParamSlider::bar(), |bar| {
                                slider.handle = bar.spawn(ParamSlider::handle(input_id)).id();
                            })
                            .id();
                    },
                )
                .id();

            slider.readout_container = container
                .container(ParamSlider::readout_container(), |readout_container| {
                    slider.readout = readout_container.label(LabelConfig::default()).id();
                })
                .id();
        });

        input
            .insert(slider)
            .insert(nih_param);

        input
    }
}
