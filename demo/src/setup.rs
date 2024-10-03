use bevy::prelude::*;
use nih_plug_bevy::param_plugin::NIHParams;
use sickle_ui::prelude::*;
use crate::widgets::param_slider::{ParamSliderConfig, UiParamSliderExt};

pub fn setup(
    mut commands: Commands,
    params: Res<NIHParams>
) {
    commands.spawn(Camera2dBundle::default());

    commands.ui_builder(UiRoot).column(|column|{
        column.row(|row|{
            row.param_slider(
                ParamSliderConfig::horizontal(
                    Some("Number 1".to_string()), 
                    0.0, 
                    100.0, 
                    true
                ),
                params.params.get("gain").unwrap().clone()
            );
        })
        .style()
        .width(Val::Percent(75.));
        column.row(|row|{
            row.slider(
                SliderConfig::horizontal(
                    Some("Number 2".to_string()), 
                    0.0, 
                    100.0, 
                    66.0, 
                    true
                )
            );
        })
        .style()
        .width(Val::Percent(60.));
    })
    .style()
    .width(Val::Percent(100.))
    .height(Val::Percent(100.))
    .justify_content(JustifyContent::Center)
    .align_items(AlignItems::Center);
}