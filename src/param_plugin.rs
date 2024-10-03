use std::sync::Arc;

use bevy::{prelude::*, utils::HashMap};
use nih_plug::{params::Params, prelude::ParamPtr};

//<========== Resources ==========>//

#[derive(Resource)]
pub struct NIHContext(Arc<dyn nih_plug::prelude::GuiContext>);

impl NIHContext {
    pub fn new(ctx: Arc<dyn nih_plug::prelude::GuiContext>) -> Self {
        Self(ctx)
    }
}

#[derive(Resource, Default)]
pub struct NIHCurrentParam(Option<Entity>);


//<========== Plugin ==========>//

pub struct NIHParamPlugin;

impl Plugin for NIHParamPlugin {
    fn build(&self, app: &mut App) {
        app
            .add_event::<ParamEvent>()
            .add_event::<ResizeRequest>()
            .add_systems(PostUpdate, param_system);
    }
}

//<========== Compnents ==========>//

#[derive(Component)]
pub struct NIHParam {
    param: ParamPtr
}

impl NIHParam {
    pub fn new(param: ParamPtr) -> Self {
        Self { param }
    }
}

//<========== Resources ==========>//

#[derive(Resource)]
pub struct NIHParams {
    pub params: HashMap<String, ParamPtr>
}

impl NIHParams {
    pub fn new(params: HashMap<String, ParamPtr>) -> Self {
        Self { params }
    }
}

//<========== Events ==========>//

pub enum ParamAction {
    Begin,
    Set(f32),
    End,
}

#[derive(Event)]
pub struct ParamEvent {
    id: Entity, 
    action: ParamAction,
}

impl ParamEvent {
    pub fn begin(id: Entity) -> Self {
        Self{
            id,
            action: ParamAction::Begin
        }
    }

    pub fn set(id: Entity, norm_val: f32) -> Self {
        Self{
            id,
            action: ParamAction::Set(norm_val)
        }
    }

    pub fn end(id: Entity) -> Self {
        Self{
            id,
            action: ParamAction::End
        }
    }
}

#[derive(Event)]
pub struct ResizeRequest;

//<========== Systems ==========>//

fn param_system(
    ctx: Res<NIHContext>,
    mut current: ResMut<NIHCurrentParam>,
    mut param_events: EventReader<ParamEvent>,
    q_nih_param: Query<&NIHParam>

) {
    for evt  in param_events.read() {
        match evt.action {
            ParamAction::Begin => {
                if current.0.is_some() {
                    panic!("A parameter is already being modified;\nCannot begin another gesture")
                }

                current.0 = Some(evt.id);
                let param_ptr = match q_nih_param.get(evt.id) {
                    Ok(nih) => nih.param,
                    Err(_) => panic!("Error while retreiving the NIH Param component.
                        \nMake sure it is present in the entity")
                };
                unsafe { ctx.0.raw_begin_set_parameter(param_ptr) };
            },
            ParamAction::Set(val) => {
                if current.0.is_none() || current.0.is_some_and(|id| id != evt.id){
                    panic!("Cannot set a parameter without starting a gesture or during another gesture")
                }

                let param_ptr = match q_nih_param.get(evt.id) {
                    Ok(nih) => nih.param,
                    Err(_) => panic!("Error while retreiving the NIH Param component.
                        \nMake sure it is present in the entity")
                };

                unsafe {
                    ctx.0.raw_set_parameter_normalized(
                        param_ptr, 
                        val
                    )
                };
            },
            ParamAction::End => {
                if current.0.is_none() || current.0.is_some_and(|id| id != evt.id){
                    panic!("Cannot end a gesture that hasn't even begun (that's mean)")
                } 

                let param_ptr = match q_nih_param.get(evt.id) {
                    Ok(nih) => nih.param,
                    Err(_) => panic!("Error while retreiving the NIH Param component.
                        \nMake sure it is present in the entity")
                };

                unsafe { ctx.0.raw_end_set_parameter(param_ptr) };

                current.0 = None;
            },
        }
    }
}